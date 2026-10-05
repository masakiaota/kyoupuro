from concurrent.futures import ThreadPoolExecutor
import json,subprocess,difflib
from pathlib import Path
import numpy as np
from build_v135_split import ROOT,RUN,BASE,SOURCE,build
from check_v113_integrated import block,normalize
from check_v089_board import compile_binary,trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save,sha,status,Geometry
from v090_data import RUN as BC_RUN

PRIOR=ROOT/'results/nn_rank/v134/20261005_prefix_diversity_studio/mechanism'

def invoke(binary,mode,path,plan,extra=None):
    args=[binary,mode,plan]+([extra] if extra else [])
    p=subprocess.run(args,input=path.read_text(),text=True,capture_output=True,check=True,timeout=90)
    return [json.loads(line) for line in p.stdout.splitlines()],p.stdout

def output(moves):
    return ''.join(f'{i} {j} {k} {"UDLR"[d]} {l}\n' for i,j,k,d,l in moves)

def snapshots(path,plan):
    geo=Geometry(path);state=geo.initial.copy();states=[state.copy()]
    for line in plan.read_text().splitlines():geo.apply(state,line);states.append(state.copy())
    return geo,states

def apply_difference(reference,row):
    s=reference.copy()
    for p,w in row['difference']:s[p]=w
    return s

def fixtures(where):
    where.mkdir(exist_ok=True)
    board=[list('.'*12) for _ in range(12)]
    for i,j,c in [(2,2,'a'),(2,1,'a'),(1,2,'a'),(1,3,'A'),(6,3,'b'),(6,6,'B'),(8,5,'c'),(8,6,'C'),(10,5,'d'),(10,6,'D')]:board[i][j]=c
    formation=[(2,1,0,3,1),(1,2,0,1,1)]
    early=[(2,2,2,3,1),(2,3,0,0,1)]
    late=[(2,2,0,3,1),(2,3,0,0,1)]
    cleanup=[(6,3,0,3,1),(6,4,0,3,1),(6,5,0,3,1),(8,5,0,3,1),(10,5,0,3,1)]
    def cell(i,j,sym):
        for _ in range(sym&3):i,j=j,11-i
        if sym&4:j=11-j
        return i,j
    result=[]
    for delay in (0,80):
        middle=[(6,3,0,3,1),(6,4,0,2,1)]*(delay//2)
        moves=formation+early+middle+late+cleanup
        for sym in range(8):
            tag=f'd{delay}_s{sym}';path=where/(tag+'.in');plan=where/(tag+'.out');edits=where/(tag+'.edits')
            transformed=[list('.'*12) for _ in range(12)]
            for i in range(12):
                for j in range(12):r,c=cell(i,j,sym);transformed[r][c]=board[i][j]
            path.write_text('12 4\n'+'\n'.join(''.join(r) for r in transformed)+'\n')
            changed=[]
            for i,j,k,d,l in moves:
                di,dj=[(-1,0),(1,0),(0,-1),(0,1)][d]
                r,c=cell(i,j,sym);qr,qc=cell(i+di*l,j+dj*l,sym)
                direction=0 if qr<r else 1 if qr>r else 2 if qc<c else 3
                changed.append((r,c,k,direction,l))
            plan.write_text(output(changed));edits.write_text(f'2 0\n{4+delay} -1\n{5+delay} -1\n')
            assert validated_output(path,plan.read_text())['E']==0
            result.append((tag,path,plan,edits))
    return result

def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True);marker=where/'result.json'
    if marker.exists():
        result=json.loads(marker.read_text());assert result['source_sha256']==sha(SOURCE);return result
    status(RUN/'pipeline','mechanism_builds')
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(SOURCE.stem,where/('solver_'+mode),local)
        compile_binary(BASE.stem,where/('solver_base_'+mode),local)
        parent=normalize(preprocess(BASE,where/('base_'+mode+'.ii'),local))
        current=normalize(preprocess(SOURCE,where/(mode+'.ii'),local))
        (where/(mode+'.patch')).write_text(''.join(difflib.unified_diff(parent.splitlines(True),current.splitlines(True))))
        aligned=current
        for key in ('struct SparseRewriteStats','class SparsePlanRewriter'):
            aligned=aligned.replace(block(aligned,key),block(parent,key),1)
        assert ' '.join(aligned.split())==' '.join(parent.split()),mode
    for label,name in [('base','check_v135_parent'),('split','check_v135_split')]:compile_binary(name,where/('probe_'+label),True)
    status(RUN/'pipeline','mechanism_fixtures')
    fixture_rows=[];atomic_count=0;step_count=0
    for tag,path,plan,edits in fixtures(where/'fixtures'):
        geo,states=snapshots(path,plan);lines=plan.read_text().splitlines()
        rows,text=invoke(where/'probe_split','roots',path,plan);(where/'fixtures'/(tag+'.roots.jsonl')).write_text(text)
        for row in rows:
            t=row['t'];alternate=lines[t].split();alternate[2]=str(row['k']);s=states[t].copy();geo.apply(s,' '.join(alternate))
            assert np.array_equal(s,apply_difference(states[t+1],row));atomic_count+=1
        rows,text=invoke(where/'probe_split','sequence',path,plan,edits);(where/'fixtures'/(tag+'.sequence.jsonl')).write_text(text)
        changed={int(t):int(k) for t,k in [l.split() for l in edits.read_text().splitlines()]}
        state=states[min(changed)].copy()
        for row in rows:
            if row['kind']=='step':
                t=row['t'];a=lines[t].split()
                if changed.get(t,0)>=0:
                    if t in changed:a[2]=str(changed[t])
                    geo.apply(state,' '.join(a))
                assert np.array_equal(state,apply_difference(states[t+1],row));step_count+=1
            else:
                final=output(row['path']);result=validated_output(path,final);assert result['E']==0 and result['T']==len(lines)-2
                (where/'fixtures'/(tag+'.shortened.out')).write_text(final)
                fixture_rows.append(dict(name=tag,span=row['span'],saved=2,**result))
    status(RUN/'pipeline','mechanism_saved_plans')
    cases=[c for c in json.loads((BC_RUN/'input_manifest.json').read_text()) if c['role']=='train'][:4]
    def one(case):
        path=BC_RUN/case['path'];plan=PRIOR/(str(case['index'])+'.out')
        assert sha(path)==case['sha256'];assert validated_output(path,plan.read_text())['E']==0
        geo,states=snapshots(path,plan);lines=plan.read_text().splitlines()
        roots,text=invoke(where/'probe_split','roots',path,plan);(where/(str(case['index'])+'.roots.jsonl')).write_text(text)
        for row in roots:
            t=row['t'];a=lines[t].split();a[2]=str(row['k']);state=states[t].copy();geo.apply(state,' '.join(a))
            assert np.array_equal(state,apply_difference(states[t+1],row))
        conditions={}
        for label in ('base','split'):
            rows,text=invoke(where/('probe_'+label),'scan',path,plan);(where/(str(case['index'])+'_'+label+'.jsonl')).write_text(text)
            assert len(rows)==len(lines)
            for row in rows:
                if row['ok']:
                    r=validated_output(path,output(row['path']));assert r['E']==0 and r['T']<len(lines)
            conditions[label]=dict(attempts=len(rows),completed=sum(r['ok'] for r in rows),
                saved=sum(len(lines)-len(r['path']) for r in rows if r['ok']),seconds=sum(r['seconds'] for r in rows),
                split_starts=sum(r['split_starts'] for r in rows),split_completed=sum(r['split_completed'] for r in rows))
        return dict(case=case['index'],input_sha256=sha(path),plan_sha256=sha(plan),roots=len(roots),conditions=conditions)
    with ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,cases))
    assert sum(r['conditions']['split']['split_starts'] for r in rows)>0
    result=dict(source_sha256=sha(SOURCE),all_legal=True,preprocessing_checked=True,fixture_rows=fixture_rows,
                atomic_root_checks=atomic_count+sum(r['roots'] for r in rows),sequence_step_checks=step_count,rows=rows)
    save(marker,result);return result
