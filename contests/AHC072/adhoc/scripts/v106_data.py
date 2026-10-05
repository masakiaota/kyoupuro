#!/usr/bin/env python3
"""NNとLNSが訪れる完成列で、通常除去集合の独立な修復結果を集める。"""
import argparse
from concurrent.futures import ProcessPoolExecutor, as_completed
import gzip
import json
from pathlib import Path
import shutil
import subprocess
import time

import numpy as np
from v089_data import ROOT, save, sha, status, now
from v090_data import RUN as BC_RUN
from v085_data import Problem, packed_text, move_hash
from v087_data import Dataset as OriginalDataset, ENCODING

RUN=ROOT/'results/nn_rank/v106/20261004_lns_selector_studio'
COLLECTOR=ROOT/'target/release/collect_v106_rewards'
EXTRACTOR=ROOT/'target/release/extract_v086_graph'
ARRAYS=('nodes','edges','mask','valid','sources','outcome','cost_ms','current_T')


def selected(phase):
    manifest=json.loads((BC_RUN/'input_manifest.json').read_text())
    train=[c for c in manifest if c['role']=='train']
    validation=[c for c in manifest if c['role']=='validation'][:64]
    count=128 if phase=='diagnostic' else 512
    result=[train[i] for i in np.linspace(0,len(train)-1,count,dtype=int)]+validation
    excluded={c['sha256'] for c in manifest if c['role']!='train'}
    excluded.update(sha(p) for p in (ROOT/'tools/in').glob('*.txt'))
    assert len(train)==4096 and len(validation)==64
    assert not any(c['sha256'] in excluded for c in result if c['role']=='train')
    assert len({c['sha256'] for c in result})==len(result)
    return result


def collect_case(task):
    root,item,identity=task;root=Path(root)
    directory=root/'cases'/f"{item['index']:06d}";directory.mkdir(parents=True,exist_ok=True)
    marker=directory/'complete.json'
    if marker.exists():
        old=json.loads(marker.read_text());assert old['input']==item and old['identity']==identity
        for name,digest in old['arrays'].items():assert sha(directory/name)==digest
        return old
    started=time.monotonic();path=BC_RUN/item['path'];assert sha(path)==item['sha256']
    text=path.read_text();problem=Problem.read(path)
    raw=directory/'raw.jsonl.gz'
    if raw.exists():
        with gzip.open(raw,'rt') as f:rows=[json.loads(line) for line in f]
    else:
        child=subprocess.run([str(COLLECTOR),str(item['index']),'check'],input=text,text=True,
                             capture_output=True,timeout=600,cwd=ROOT)
        (directory/'collector.log').write_text(child.stderr)
        if child.returncode:raise RuntimeError(f"collector {item['index']}: {child.returncode}; {directory/'collector.log'}")
        rows=[json.loads(line) for line in child.stdout.splitlines()]
        with gzip.open(raw,'wt') as f:f.write(child.stdout)
    count=len(rows);M=item['M'];assert 1<=count<=4 and rows[0]['iteration']==0
    assert [r['row'] for r in rows]==list(range(count))
    assert len({r['iteration'] for r in rows})==count
    stats=dict(groups=count,candidates=0,trials=0,successful=0,positive=0,failed_extract=0,failed_insert=0,
               independent_replays=0,duplicate_states=count-len({move_hash(r['current']) for r in rows}))
    arrays={'mask':np.zeros((count,32,M),bool),'valid':np.zeros((count,32),bool),
            'sources':np.zeros((count,32),np.uint8),'outcome':np.zeros((count,32,8),np.int32),
            'cost_ms':np.zeros((count,32,8),np.float32),'current_T':np.array([r['current_T'] for r in rows],np.int32)}
    for i,row in enumerate(rows):
        assert row['current_T']==len(row['current'])
        assert problem.replay(packed_text(problem,row['current']))['E']==0
        stats['independent_replays']+=1
        sets=set()
        for j,c in enumerate(row['candidates']):
            ids=c['ids'];assert 1<=len(ids)<=12 and len(set(ids))==len(ids) and min(ids)>=0 and max(ids)<M
            assert tuple(ids) not in sets;sets.add(tuple(ids))
            values=np.array(c['outcome_T']);cost=np.array(c['milliseconds'])
            assert values.shape==(8,) and np.all((values>0)|(values==-1)|(values==-2))
            assert np.isfinite(cost).all() and (cost>=0).all()
            assert all((t>0)==(h!=0) for t,h in zip(values,c['hashes']))
            arrays['mask'][i,j,ids]=True;arrays['valid'][i,j]=True;arrays['sources'][i,j]=c['sources']
            arrays['outcome'][i,j]=values;arrays['cost_ms'][i,j]=cost
            stats['candidates']+=1;stats['trials']+=8;stats['successful']+=int((values>0).sum())
            stats['positive']+=int(((values>0)&(values<row['current_T'])).sum())
            stats['failed_extract']+=int((values==-1).sum());stats['failed_insert']+=int((values==-2).sum())
        assert 1<=len(sets)<=32
        assert sum(bool(c['sources']&4) for c in row['candidates'])==1
        assert row['candidates'][row['baseline_index']]['sources']&4
    assert rows[0]['check_passed'] is True and rows[0]['audit_plan'] is not None
    assert problem.replay(packed_text(problem,rows[0]['audit_plan']))['E']==0
    stats['independent_replays']+=1
    request=[text.rstrip(),str(count)]
    for r in rows:
        request.append(str(len(r['current'])));request.extend(' '.join(map(str,m)) for m in r['current'])
    child=subprocess.run([str(EXTRACTOR)],input=('\n'.join(request)+'\n').encode(),capture_output=True,check=True,timeout=60,cwd=ROOT)
    (directory/'extract.log').write_bytes(child.stderr)
    values=np.frombuffer(child.stdout,np.float32);width=M*24+5*M*M
    assert values.size==count*width and np.isfinite(values).all()
    values=values.reshape(count,width)
    arrays['nodes']=values[:,:M*24].reshape(count,M,24).copy()
    arrays['edges']=values[:,M*24:].reshape(count,5,M,M).copy()
    sums=arrays['edges'].sum(-1);assert np.all((abs(sums-1)<2e-6)|(sums==0))
    assert not np.diagonal(arrays['edges'],axis1=-2,axis2=-1).any()
    hashes={}
    for name,array in arrays.items():
        dest=directory/(name+'.npy');np.save(dest,array);hashes[dest.name]=sha(dest)
    result=dict(input=item,identity=identity,arrays=hashes,statistics=stats,iterations=[r['iteration'] for r in rows],
                seconds=time.monotonic()-started,raw_sha256=sha(raw),completed_at=now())
    save(marker,result);return result


def collect(root,phase,workers,limit=0):
    root.mkdir(parents=True,exist_ok=True)
    identity=dict(collector_sha256=sha(COLLECTOR),extractor_sha256=sha(EXTRACTOR),
                  source_sha256=sha(ROOT/'adhoc/bin/collect_v106_rewards.cpp'),
                  base_sha256=sha(ROOT/'adhoc/bin/v106_probe_base.cpp'),seed=106001)
    config=root/'collection_config.json'
    if config.exists():assert json.loads(config.read_text())['identity']==identity
    else:
        save(config,dict(identity=identity,workers=workers,phase=phase,created_at=now()))
        for p in (COLLECTOR,EXTRACTOR):shutil.copy2(p,root/(p.name+'_frozen'))
    cases=selected(phase)
    if limit:cases=cases[:limit]
    save(root/('pilot_manifest.json' if limit else 'input_manifest.json'),cases)
    started=time.monotonic();results=[]
    with ProcessPoolExecutor(max_workers=workers) as pool:
        futures=[pool.submit(collect_case,(str(root),c,identity)) for c in cases]
        for f in as_completed(futures):
            results.append(f.result())
            if len(results)%8==0 or len(results)==len(cases):
                elapsed=time.monotonic()-started
                status(root,'collecting',completed=len(results),total=len(cases),seconds=elapsed,
                       remaining_seconds=elapsed*(len(cases)-len(results))/len(results))
    results.sort(key=lambda c:c['input']['index'])
    stats={key:sum(r['statistics'][key] for r in results) for key in results[0]['statistics']}
    output=dict(cases=results,statistics=stats,identity=identity,phase=phase,seconds=time.monotonic()-started,completed_at=now())
    save(root/('pilot.json' if limit else 'dataset.json'),output)
    status(root,'data_ready',**stats,seconds=output['seconds'])


class Dataset(OriginalDataset):
    def __init__(self,run):
        self.run=run;self.description=json.loads((run/'dataset.json').read_text())
        original=json.loads((ENCODING/'dataset.json').read_text())
        self.mean=np.array(original['mean'],np.float32);self.scale=np.array(original['scale'],np.float32)
        self.arrays={};self.rows=[];self.splits={'train':[],'validation':[]};self.input_counts={'train':0,'validation':0}
        for case in self.description['cases']:
            item=case['input'];index=item['index'];count=case['statistics']['groups']
            self.input_counts[item['role']]+=1;folder=run/'cases'/f'{index:06d}'
            for name,digest in case['arrays'].items():assert sha(folder/name)==digest
            self.arrays[index]={name:np.load(folder/(name+'.npy'),mmap_mode='r') for name in ARRAYS}
            for row in range(count):
                self.splits[item['role']].append(len(self.rows));self.rows.append((index,row,item['M'],count))


def teacher_quality(root):
    data=Dataset(root);result={}
    for role,indices in data.splits.items():
        per_case={};xs=[];ys=[]
        for i in indices:
            raw=data.batch([i]);n=int(raw['candidate_valid'][0].sum());gain=raw['gains'][0,:n]
            fit=gain[:,:4].mean(-1);audit=gain[:,4:].mean(-1)
            baseline=int(((raw['sources'][0,:n]&4)!=0).argmax());chosen=int(fit.argmax())
            item=per_case.setdefault(int(raw['cases'][0]),[])
            item.append(dict(gain=float(audit[chosen]-audit[baseline]),baseline=float(audit[baseline]),teacher=float(audit[chosen])))
            xs.extend((fit-fit.mean()).tolist());ys.extend((audit-audit.mean()).tolist())
        diffs=np.array([np.mean([r['gain'] for r in rows]) for rows in per_case.values()])
        draws=np.random.default_rng(106002).integers(0,len(diffs),(6000,len(diffs)))
        corr=float(np.corrcoef(xs,ys)[0,1]) if np.std(xs)>0 and np.std(ys)>0 else None
        result[role]=dict(inputs=len(per_case),groups=len(indices),additional_gain=float(diffs.mean()),
                          bootstrap95=np.percentile(diffs[draws].mean(1),[2.5,97.5]).tolist(),
                          centered_correlation=corr,per_input=per_case)
    result['gate_passed']=result['validation']['additional_gain']>=.10
    result['completed_at']=now();save(root/'teacher_quality.json',result)
    print(json.dumps({k:v if not isinstance(v,dict) else {a:b for a,b in v.items() if a!='per_input'} for k,v in result.items()}),flush=True)
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN/'diagnostic')
    p.add_argument('--phase',choices=('diagnostic','main'),default='diagnostic');p.add_argument('--workers',type=int,default=16)
    p.add_argument('--limit',type=int,default=0);p.add_argument('--quality',action='store_true');a=p.parse_args()
    if a.quality:teacher_quality(a.run)
    else:collect(a.run,a.phase,a.workers,a.limit)
