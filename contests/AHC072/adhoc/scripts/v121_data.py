#!/usr/bin/env python3
"""短縮前後を採取し、記録した経路内の最小完成費用へラベルをそろえる。"""
from collections import deque
from concurrent.futures import ThreadPoolExecutor,as_completed
from pathlib import Path
import hashlib,json,os,shutil,subprocess,time
import numpy as np
from v116_data import ROOT,BC,load,save,sha,now
from analyze_v119_saved import board,height

RUN=ROOT/'results/nn_rank/v121/20261004_learned_finite_studio'
PILOT=ROOT/'results/nn_rank/v119/20261004_matched_pairs_studio'

def selected():
    available=sorted((x for x in load(BC/'input_manifest.json') if x['role']=='train'),key=lambda x:x['index'])
    assert len(available)==4096
    train=[available[i] for i in np.linspace(0,4095,1024,dtype=int)]
    used={x['index'] for x in train};rest=[x for x in available if x['index'] not in used]
    dev=[rest[i] for i in np.linspace(0,len(rest)-1,128,dtype=int)]
    excluded={x['sha256'] for x in load(BC/'input_manifest.json') if x['role']!='train'}
    for name in ['in','validation1']:excluded.update(sha(p) for p in (ROOT/'tools'/name).glob('*.txt'))
    result=[dict(x,guidance_role=role) for role,group in [('train',train),('development',dev)] for x in group]
    assert len({x['sha256'] for x in result})==len(result) and not any(x['sha256'] in excluded for x in result)
    return result

def harvest(item,identity):
    case=RUN/'data/cases'/f"{item['index']:06d}";case.mkdir(parents=True,exist_ok=True)
    marker=case/'harvest.json'
    if marker.exists():
        old=load(marker);assert old['identity']==identity and old['input']==item
        for name,digest in old['sha256'].items():assert sha(case/name)==digest
        return old
    inp=BC/item['path'];assert sha(inp)==item['sha256']
    old_case=PILOT/'cases'/case.name;reused=False;started=time.monotonic()
    names=['pairs.txt','teachers.jsonl','samples.raw','rejections.jsonl','answer.txt']
    if (old_case/'complete.json').exists():
        old=load(old_case/'complete.json')
        assert old['input']==item
        for key in ['recorder_sha256','checker_sha256','parent_sha256']:assert old['identity'][key]==identity[key]
        for name in names:
            assert sha(old_case/name)==old['sha256'][name];shutil.copy2(old_case/name,case/name)
        stats=old['statistics'];reused=True
    else:
        if not (case/'pairs.txt').exists():
            env=dict(os.environ,V119_PAIRS=str(case/'pairs.partial.txt'))
            p=subprocess.run([RUN/'frozen/recorder'],input=inp.read_bytes(),capture_output=True,env=env,cwd=ROOT,timeout=30)
            (case/'solver.err').write_bytes(p.stderr);(case/'answer.txt').write_bytes(p.stdout)
            assert p.returncode==0 and b'LNS diagnostic:' not in p.stderr,(item['index'],p.returncode,p.stderr[-1000:])
            (case/'pairs.partial.txt').replace(case/'pairs.txt')
        p=subprocess.run([RUN/'frozen/checker',inp,case/'pairs.txt',case],capture_output=True,text=True,cwd=ROOT,timeout=120)
        (case/'checker.err').write_text(p.stderr)
        assert p.returncode==0,(item['index'],p.returncode,p.stderr[-1000:]);stats=json.loads(p.stdout)
    values=np.fromfile(case/'samples.raw',dtype='<f4').reshape(-1,52)
    assert len(values)==stats.get('labels',0) and np.isfinite(values).all()
    result=dict(input=item,identity=identity,statistics=stats,reused=reused,seconds=time.monotonic()-started,
                sha256={name:sha(case/name) for name in names},completed_at=now())
    save(marker,result);return result

def replay(start,path,nest,ray,goal=None):
    a=list(start);states=[tuple(a)]
    for p,k,d,length in path:
        q=ray[p,d,length];assert 0<=k<height(a[p]) and length<=k+1
        packet=a[p]>>(4*k);a[p]&=(1<<(4*k))-1
        assert height(a[q])+height(packet)<=8
        reverse=0
        while packet:reverse=(reverse<<4)|(packet&15);packet>>=4
        a[q]|=reverse<<(4*height(a[q]))
        for c in [p,q]:
            while a[c] and (a[c]>>(4*(height(a[c])-1)))==nest[c]:a[c]&=(1<<(4*(height(a[c])-1)))-1
        states.append(tuple(a))
    if goal is not None:assert states[-1]==tuple(goal)
    return states

def graph_rows(teacher,rows,nest,ray,group):
    nodes=[];index={};features=[];edges=[];old_labels={};observations=0
    for role,key in [(0,'before_path'),(1,'after_path')]:
        path=teacher[key];states=replay(teacher['initial'],path,nest,ray,teacher['goal'])
        part=rows[(rows[:,50]==teacher['teacher'])&(rows[:,51]==role)]
        assert len(part)==len(states)
        ids=[]
        for i,state in enumerate(states):
            assert part[i,48]==len(path)-i and part[i,49]==i
            if state not in index:index[state]=len(nodes);nodes.append(state);features.append(part[i,:48]);old_labels[index[state]]=[]
            at=index[state];assert np.array_equal(features[at],part[i,:48]);ids.append(at)
            old_labels[at].append(int(part[i,48]));observations+=1
        for i,move in enumerate(path):edges.append((ids[i],ids[i+1],move))
    root=index[tuple(teacher['initial'])];goal=index[tuple(teacher['goal'])]
    outgoing=[[] for _ in nodes];incoming=[[] for _ in nodes]
    for eid,(a,b,_) in enumerate(edges):outgoing[a].append(eid);incoming[b].append(eid)
    def distances(start,reverse):
        cost=[10**8]*len(nodes);step=[-1]*len(nodes);q=deque([start]);cost[start]=0
        while q:
            v=q.popleft()
            for eid in (incoming if reverse else outgoing)[v]:
                a,b,_=edges[eid];to=a if reverse else b
                if cost[to]>cost[v]+1:cost[to]=cost[v]+1;step[to]=eid;q.append(to)
        return cost,step
    remain,step=distances(goal,True);prefix,_=distances(root,False)
    assert remain[root]<=teacher['after'] and all(x<10**8 for x in remain+prefix)
    out=[];reduced=0
    for at,state in enumerate(nodes):
        suffix=[];cur=at
        while cur!=goal:
            eid=step[cur];assert eid>=0 and edges[eid][0]==cur
            _,cur,move=edges[eid];suffix.append(move);assert len(suffix)<=len(nodes)
        replay(state,suffix,nest,ray,teacher['goal'])
        assert len(suffix)==remain[at] and features[at][0]<=remain[at]
        reduced+=sum(x>remain[at] for x in old_labels[at])
        out.append([*features[at],remain[at],prefix[at],group])
    return out,dict(nodes=len(nodes),observations=observations,labels_lowered=reduced,root_cost=remain[root])

def common_prefix(a,b):
    n=0
    while a and b and (a&15)==(b&15):a>>=4;b>>=4;n+=1
    return n

def write_problems(path,teachers,ray):
    out=[str(len(teachers))]
    for t in teachers:
        moves=t['before_path'];cells=sorted({c for p,k,d,l in moves for c in [p,ray[p,d,l]]})
        assert len(cells)==t['cells']<=8
        sources=sum(height(t['initial'][c])>common_prefix(t['initial'][c],t['goal'][c]) for c in cells)
        destinations=sum(height(t['goal'][c])>common_prefix(t['initial'][c],t['goal'][c]) for c in cells)
        towers=sum(t['initial'][c]!=0 for c in cells)
        # has_nestは探索の状態遷移に使わない。対象抽出は凍結した採取器が既に確認している。
        out.append(f"0 0 {len(moves)-1} {len(cells)} {t['pieces']} {towers} {len(moves)-max(sources,destinations)} 1 {len(moves)}")
        out.extend(f"{c} {t['initial'][c]} {t['goal'][c]}" for c in cells)
        out.append(' '.join(map(str,t['initial'])))
        out.extend(' '.join(map(str,move)) for move in moves)
    path.write_text('\n'.join(out)+'\n')

def prepare(items):
    arrays={role:[] for role in ['train','development']};weights={role:[] for role in arrays};groups={role:0 for role in arrays}
    audits=[]
    for item in items:
        case=RUN/'data/cases'/f"{item['index']:06d}";values=np.fromfile(case/'samples.raw',dtype='<f4').reshape(-1,52)
        _,nest,ray=board(BC/item['path']);teachers=[json.loads(x) for x in (case/'teachers.jsonl').read_text().splitlines() if x]
        teachers=[t for t in teachers if t['teacher']<64];role=item['guidance_role']
        write_problems(case/'problems.txt',teachers,ray)
        for teacher in teachers:
            rows,audit=graph_rows(teacher,values,nest,ray,groups[role]);arrays[role].extend(rows)
            weight=4. if teacher['after']<teacher['baseline'] else 1.;weights[role].extend([weight]*len(rows))
            audits.append(dict(case=item['index'],teacher=teacher['teacher'],role=role,group=groups[role],weight=weight,**audit));groups[role]+=1
    hashes={};counts={}
    for role in arrays:
        data=np.asarray(arrays[role],dtype=np.float32);weight=np.asarray(weights[role],dtype=np.float32)
        assert data.ndim==2 and data.shape[1]==51 and np.isfinite(data).all()
        np.save(RUN/'data'/f'{role}.npy',data);np.save(RUN/'data'/f'{role}_weights.npy',weight)
        hashes[role]=sha(RUN/'data'/f'{role}.npy');hashes[role+'_weights']=sha(RUN/'data'/f'{role}_weights.npy')
        counts[role]=dict(rows=len(data),groups=groups[role],hard_rows=int((weight==4).sum()))
    result=dict(columns=51,arrays=hashes,counts=counts,graph_audits=audits,completed_at=now())
    save(RUN/'data/dataset.json',result);return result

def main():
    items=selected();identity={key:load(PILOT/'identity.json')[key] for key in ['parent_sha256','recorder_sha256','checker_sha256']}
    for name in ['recorder','checker']:assert sha(RUN/'frozen'/name)==identity[name+'_sha256']
    manifest=RUN/'input_manifest.json'
    if manifest.exists():assert load(manifest)==items
    else:save(manifest,items)
    started=time.monotonic();results=[]
    with ThreadPoolExecutor(max_workers=12) as pool:
        for f in as_completed([pool.submit(harvest,x,identity) for x in items]):
            results.append(f.result());elapsed=time.monotonic()-started
            save(RUN/'data/status.json',dict(stage='harvesting',completed=len(results),total=len(items),seconds=elapsed,remaining_seconds=elapsed*(len(items)-len(results))/len(results),updated_at=now()))
    stats={}
    for item in results:
        for k,v in item['statistics'].items():stats[k]=stats.get(k,0)+v
    save(RUN/'data/collection.json',dict(cases=len(items),reused=sum(x['reused'] for x in results),statistics=stats,seconds=time.monotonic()-started,completed_at=now()))
    save(RUN/'data/status.json',dict(stage='preparing_graph_labels',updated_at=now()))
    dataset=prepare(items)
    save(RUN/'data/status.json',dict(stage='completed',counts=dataset['counts'],seconds=time.monotonic()-started,updated_at=now()))
    print(json.dumps(dict(statistics=stats,counts=dataset['counts'],seconds=time.monotonic()-started),indent=2))

if __name__=='__main__':main()
