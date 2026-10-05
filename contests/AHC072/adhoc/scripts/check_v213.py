#!/usr/bin/env python3
"""v213のビルド・固定・機構検査・100件評価。analyzeは保存結果だけを読む。"""
import argparse
from collections import Counter
import csv
from datetime import datetime
import difflib
import json
import os
from pathlib import Path
import subprocess
import sys

import build_v202_v203 as builder
from build_v202_v203 import save, sha
from check_v037_results import verify_output, log_values
from tune_v204 import evaluator, mechanism

ROOT=Path(__file__).resolve().parents[2]
AUDIT=ROOT/'adhoc/v213_audit'
OUT=ROOT/'results/analysis/v213'
PARENT='v210_collection_color_quotient'
CHILD='v213_atomic_color_runs'
PARENT_RUN='20261003T113327+0900_v210_collection_color_quotient_5ad8ef'
LABEL='v213_atomic_color_runs_all_phases_same_budget'


def status(state,**extra):
    OUT.mkdir(parents=True,exist_ok=True)
    save(OUT/'status.json',dict(state=state,pid=os.getpid(),updated_at=datetime.now().astimezone().isoformat(),**extra))


def build():
    assert not (AUDIT/'frozen.json').exists()
    AUDIT.mkdir(exist_ok=True)
    before=(ROOT/f'src/bin/{PARENT}.cpp').read_text()
    after=(ROOT/f'src/bin/{CHILD}.cpp').read_text()
    changes=[];a=before.splitlines(True);b=after.splitlines(True);offset=0;current=before
    for tag,i,j,k,l in difflib.SequenceMatcher(None,a,b,autojunk=False).get_opcodes():
        if tag=='equal':continue
        old,new=''.join(a[i:j]),''.join(b[k:l]);at=len(''.join(a[:i]))+offset
        assert current[at:at+len(old)]==old
        changes.append(dict(old=old,new=new,offset=at))
        current=current[:at]+new+current[at+len(old):];offset+=len(new)-len(old)
    assert current==after
    save(AUDIT/f'{CHILD}_changes.json',changes)
    (AUDIT/'parent.diff').write_text(''.join(difflib.unified_diff(a,b,fromfile=PARENT,tofile=CHILD)))
    (AUDIT/'preregistration.md').write_bytes((ROOT/'notes/experiments/v213.md').read_bytes())
    start,end='struct TimeKeeper {','struct Rng {'
    assert before[before.index(start):before.index(end)]==after[after.index(start):after.index(end)]
    assert '#define LOCAL_SECONDS(value) (PROGRAM_TIME_LIMIT_SEC * ((value) / JUDGE_TIME_LIMIT_SEC))' in after
    args=argparse.Namespace(bin_name=CHILD,jobs=2,wait_lock=True)
    with evaluator.acquire_eval_lock(args,'v213_build'):
        builder.AUDIT=AUDIT;builder.PAIRS=[(PARENT,CHILD)];builder.PARENT_RUNS={PARENT:PARENT_RUN}
        if (AUDIT/'preflight.json').exists():
            (AUDIT/'preflight.json').unlink()
        builder.main()
        env=os.environ.copy()
        if sys.platform=='darwin':
            env.setdefault('SDKROOT',subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip())
            env.setdefault('MACOSX_DEPLOYMENT_TARGET','15.0')
        cmd=[env.get('CXX','g++-15'),*builder.FLAGS,'-DLOCAL',str(ROOT/'adhoc/bin/check_v213_atomic_runs.cpp'),'-o',str(AUDIT/'check_atomic')]
        r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True)
        (AUDIT/'probe_build.log').write_text(r.stdout+r.stderr)
        assert r.returncode==0 and not r.stderr,r.stderr
    status('built',solver_executions=0)
    print('Both modes and checker built; solver executions=0',flush=True)


def rows():
    return [json.loads(line) for line in (ROOT/'results/eval_records.jsonl').open() if CHILD in line or PARENT_RUN in line]


def freeze():
    assert not (AUDIT/'frozen.json').exists()
    pre=json.loads((AUDIT/'preflight.json').read_text())
    for name,digest in pre['solver_sha256'].items():assert sha(ROOT/f'src/bin/{name}.cpp')==digest
    for b in pre['builds']:
        assert not b['warnings'] and sha(AUDIT/f"{CHILD}_{b['mode']}")==b['binary_sha256']
    assert not (AUDIT/'probe_build.log').read_text()
    inputs=evaluator.list_input_files(ROOT/'tools/in');assert len(inputs)==100
    records=rows();assert not any(r['bin']==CHILD for r in records)
    parents=sorted((r for r in records if r['run_id']==PARENT_RUN),key=lambda r:r['case_name'])
    assert len(parents)==100
    fixed=[*inputs,ROOT/f'src/bin/{PARENT}.cpp',ROOT/f'src/bin/{CHILD}.cpp',Path(__file__),
           ROOT/'adhoc/scripts/prepare_v213.py',ROOT/'adhoc/bin/check_v213_atomic_runs.cpp',
           ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh']
    fixed.extend(p for p in AUDIT.iterdir() if p.is_file())
    for name in ('build_v202_v203.py','make_v202_v203.py','tune_v204.py','tune_v073.py','check_v037_results.py','check_v028_two_orders.py'):
        fixed.append(ROOT/'adhoc/scripts'/name)
    for r in parents:
        p=ROOT/r['stdout_path'];assert verify_output(str(ROOT/'tools/in'/r['case_name']),p)==r['score']
        fixed.extend((p,p.with_suffix('.txt.err')))
    source=AUDIT/'reference_source.csv';source.write_bytes((ROOT/'results/score_detail.csv').read_bytes())
    refs=list(csv.DictReader(source.open()))
    reference={p.name:min(float(r[p.name]) for r in refs if r.get(p.name) and float(r[p.name])>0) for p in inputs}
    fixed.append(source)
    save(AUDIT/'frozen.json',dict(sha256={str(p.relative_to(ROOT)):sha(p) for p in fixed},parents=parents,
        reference=reference,jobs=2,local_time_ratio=0.8,lns_end_ms=1520,search_limit_ms=1544,solver_executions=0))
    status('frozen',solver_executions=0);print('Sources, binaries, checker, parent, references and 100 inputs frozen',flush=True)


def frozen():
    data=json.loads((AUDIT/'frozen.json').read_text())
    for p,digest in data['sha256'].items():assert sha(ROOT/p)==digest,p
    return data


def no_splits(case,answer):
    lines=(ROOT/'tools/in'/case).read_text().splitlines();N,K=map(int,lines[0].split())
    grid=''.join(lines[1:N+1]);a=[[ord(c)-ord('a')+1] if 'a'<=c<='l' else [] for c in grid]
    nests={p:ord(c)-ord('A')+1 for p,c in enumerate(grid) if 'A'<=c<='L'}
    direction={'U':(-1,0),'D':(1,0),'L':(0,-1),'R':(0,1)}
    total_runs=total_slimes=joins=0
    for t,line in enumerate(answer.read_text().splitlines()):
        i,j,k,d,l=line.split();i,j,k,l=map(int,(i,j,k,l));p=i*N+j;dr,dc=direction[d];q=(i+dr*l)*N+j+dc*l
        assert 0<=k<len(a[p]) and (k==0 or a[p][k-1]!=a[p][k]),(case,t+1,'same color split')
        total_slimes+=len(a[p]);total_runs+=1+sum(x!=y for x,y in zip(a[p],a[p][1:]))
        parcel=a[p][k:][::-1];joins+=bool(a[q]) and a[q][-1]==parcel[0]
        a[p]=a[p][:k];a[q].extend(parcel)
        for v in (p,q):
            while a[v] and a[v][-1]==nests.get(v,-1):a[v].pop()
    assert not any(a)
    return dict(departure_runs=total_runs,departure_slimes=total_slimes,joins=joins,same_color_splits=0)


def diagnostic():
    frozen();assert not (AUDIT/'diagnostic_started.json').exists()
    args=argparse.Namespace(bin_name=CHILD,jobs=2,wait_lock=True)
    with evaluator.acquire_eval_lock(args,'v213_diagnostic'):
        save(AUDIT/'diagnostic_started.json',dict(case='0000.txt',modes=['local','production']))
        status('diagnostic')
        with (ROOT/'tools/in/0000.txt').open() as stdin,(AUDIT/'partial.log').open('w') as out:
            subprocess.run([str(AUDIT/'check_atomic')],stdin=stdin,stdout=out,stderr=subprocess.STDOUT,check=True,timeout=120,cwd=ROOT)
        partial=json.loads((AUDIT/'partial.log').read_text());assert partial['passed']
        print(json.dumps(partial),flush=True)
        smoke=[]
        for mode in ('local','production'):
            answer=AUDIT/f'{mode}_0000.txt'
            with (ROOT/'tools/in/0000.txt').open() as stdin,answer.open('w') as stdout,answer.with_suffix('.txt.err').open('w') as stderr:
                subprocess.run([str(AUDIT/f'{CHILD}_{mode}')],stdin=stdin,stdout=stdout,stderr=stderr,check=True,timeout=15,cwd=ROOT)
            T=verify_output(str(ROOT/'tools/in/0000.txt'),answer);detail=no_splits('0000.txt',answer)
            assert 'diagnostic:' not in answer.with_suffix('.txt.err').read_text()
            if mode=='local':
                c,t=mechanism(ROOT/'tools/in/0000.txt',answer,(5,15))
                assert c['atomic_dp_positions']>c['atomic_dp_boundaries']
                assert c['atomic_router_join_stops']>0
                assert c['atomic_run_reinserted_slimes']>c['atomic_run_reinsertions']>0
                detail.update(counts=c,times_ms=t)
            smoke.append(dict(mode=mode,T=T,detail=detail))
            print(f'{mode}: legal, E=0, same-color splits=0',flush=True)
        save(AUDIT/'diagnostic.json',dict(passed=True,partial=partial,smoke=smoke));status('diagnostic_passed')


def analyze():
    fix=frozen();current=sorted((r for r in rows() if r['bin']==CHILD),key=lambda r:r['case_name'])
    assert len(current)==100 and len({r['run_id'] for r in current})==1
    parent={r['case_name']:r for r in fix['parents']}
    counts,old_counts,times,old_times=Counter(),Counter(),Counter(),Counter();cases=[]
    for r in current:
        assert r['status']=='ok' and r['local'] and r['input_dir']=='tools/in' and r['label']==LABEL
        p=ROOT/r['stdout_path'];c,t=mechanism(ROOT/'tools/in'/r['case_name'],p,(5,15));assert c['T']==r['score']
        detail=no_splits(r['case_name'],p);assert c['atomic_output_same_color_splits']==0
        old=parent[r['case_name']];oc,ot,_=log_values((ROOT/old['stdout_path']).with_suffix('.txt.err'))
        counts.update(c);old_counts.update(oc);times.update(t);old_times.update(ot)
        relative=100*fix['reference'][r['case_name']]/r['score'];old_relative=100*fix['reference'][r['case_name']]/old['score']
        cases.append(dict(case=r['case_name'],score=r['score'],parent_score=old['score'],delta=r['score']-old['score'],
            elapsed_ms=r['elapsed'],relative=relative,parent_relative=old_relative,relative_delta=relative-old_relative,**detail,
            counts=c,times_ms=t))
    maximum=max(r['elapsed_ms'] for r in cases);total=sum(r['score'] for r in cases);old_total=sum(r['parent_score'] for r in cases)
    active=(counts['atomic_dp_positions']>counts['atomic_dp_boundaries'] and counts['atomic_router_join_stops']>0 and
            counts['atomic_run_reinserted_slimes']>counts['atomic_run_reinsertions']>0)
    assert active
    result=dict(bin=CHILD,run_id=current[0]['run_id'],cases=100,total=total,parent_total=old_total,average=total/100,
        delta=total-old_total,delta_percent=100*(total/old_total-1),
        relative_average=sum(r['relative'] for r in cases)/100,parent_relative_average=sum(r['parent_relative'] for r in cases)/100,
        relative_delta=sum(r['relative_delta'] for r in cases)/100,wins=sum(r['delta']<0 for r in cases),
        draws=sum(r['delta']==0 for r in cases),losses=sum(r['delta']>0 for r in cases),max_elapsed_ms=maximum,
        average_elapsed_ms=sum(r['elapsed_ms'] for r in cases)/100,
        counts=dict(counts),parent_counts=dict(old_counts),average_times_ms={k:v/100 for k,v in times.items()},
        parent_average_times_ms={k:v/100 for k,v in old_times.items()},all_legal=True,E=0,same_color_splits=0,mechanism_passed=active,
        lns_attempts_per_second=1000*counts['lns_attempts']/times['temporal_lns'],
        parent_lns_attempts_per_second=1000*old_counts['lns_attempts']/old_times['temporal_lns'],
        adopted=total<old_total and maximum<=2000)
    save(OUT/'comparison.json',result);save(OUT/'cases.json',cases)
    with (OUT/'cases.csv').open('w',newline='') as stream:
        cols=[k for k in cases[0] if k not in ('counts','times_ms')];writer=csv.DictWriter(stream,fieldnames=cols,extrasaction='ignore')
        writer.writeheader();writer.writerows(cases)
    print(json.dumps({k:result[k] for k in ('run_id','total','parent_total','delta','delta_percent','relative_delta','wins','draws','losses',
        'max_elapsed_ms','lns_attempts_per_second','parent_lns_attempts_per_second','adopted')},indent=2),flush=True)
    return result


def run():
    frozen();assert json.loads((AUDIT/'diagnostic.json').read_text())['passed']
    assert not (OUT/'evaluation_started.json').exists()
    args=argparse.Namespace(bin_name=CHILD,jobs=evaluator.default_jobs(),wait_lock=True,dry_run=False,verbose=False,label=LABEL,no_local=False)
    with evaluator.acquire_eval_lock(args,'tools/in'):
        save(OUT/'evaluation_started.json',dict(cases=100,label=LABEL));status('evaluating')
        assert evaluator.run_eval_locked(args,evaluator.list_input_files(ROOT/'tools/in'),'tools/in',True,True)==0
    result=analyze();status('completed',adopted=result['adopted'])


if __name__=='__main__':
    try:
        {'build':build,'freeze':freeze,'diagnostic':diagnostic,'run':run,'analyze':analyze}[sys.argv[1]]()
    except BaseException as e:
        status('failed',error=f'{type(e).__name__}: {e}');raise
