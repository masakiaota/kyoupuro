#!/usr/bin/env python3
"""対応する複数乱数で測った集合の実測利益を保存する。"""
import argparse
from concurrent.futures import ProcessPoolExecutor, as_completed
import gzip
import json
from pathlib import Path
import shutil
import subprocess
import time

import numpy as np
from check_v082_relation import replay
from v086_data import SOURCE, ROOT, examples, save, sha, status, now

ENCODING=ROOT/'results/nn_rank/v086/20261002T181701_studio'


def collect_case(task):
    run,item,binary,digest=task;run=Path(run);binary=Path(binary)
    folder=run/'cases'/f"{item['index']:06d}"
    if (folder/'complete.json').exists():
        result=json.loads((folder/'complete.json').read_text());assert result['binary_sha256']==digest
        for key,value in result['arrays'].items():assert sha(folder/key)==value
        return result
    folder.mkdir(parents=True,exist_ok=True)
    if (folder/'raw.jsonl').exists() or (folder/'raw.jsonl.gz').exists():
        raise RuntimeError(f'incomplete input requires inspection: {folder}')
    started=time.monotonic();text,ids,rows,paths=examples(item)
    count=len(rows);M=len(ids)
    if count:
        parts=[text.rstrip(),str(count)]
        for i,row in enumerate(rows):
            parts.append(f"{i} {len(row['before'])}");parts.extend(' '.join(map(str,m)) for m in row['before'])
            parts.append(str(len(row['selected_ids']))+' '+' '.join(map(str,row['selected_ids'])))
        request=folder/'request.txt';request.write_text('\n'.join(parts)+'\n')
        with request.open('rb') as inp,(folder/'raw.jsonl').open('wb') as out,(folder/'collector.log').open('wb') as err:
            subprocess.run([str(binary),str(item['index']),'check' if item['index']<4 else 'collect'],
                           stdin=inp,stdout=out,stderr=err,check=True,cwd=ROOT,timeout=7200)
    arrays={'mask':np.zeros((count,64,M),bool),'valid':np.zeros((count,64),bool),'sources':np.zeros((count,64),np.uint8),
            'outcome':np.zeros((count,64,8),np.int32),'cost_ms':np.zeros((count,64,8),np.float32),
            'current_T':np.array([len(row['before']) for row in rows],np.int32)}
    stats={'groups':count,'candidates':0,'trials':0,'successful':0,'positive':0,'failed_extract':0,'failed_insert':0,
           'independent_replays':0,'repeat_checked':False}
    seen=set()
    if count:
        with (folder/'raw.jsonl').open() as raw,gzip.open(folder/'raw.jsonl.gz','wt') as compressed:
            for line in raw:
                compressed.write(line);entry=json.loads(line);i=entry['row'];assert i not in seen;seen.add(i)
                assert entry['current_T']==arrays['current_T'][i]
                candidates=entry['candidates'];assert 1<=len(candidates)<=64
                sets=set()
                for j,candidate in enumerate(candidates):
                    selected=candidate['ids'];assert 1<=len(selected)<=12 and len(selected)==len(set(selected))
                    assert min(selected)>=0 and max(selected)<M and tuple(selected) not in sets;sets.add(tuple(selected))
                    result=np.asarray(candidate['outcome_T']);assert result.shape==(8,) and np.all((result>0)|(result==-1)|(result==-2))
                    cost=np.asarray(candidate['milliseconds']);assert np.isfinite(cost).all() and (cost>=0).all()
                    assert all((t>0)==(h!=0) for t,h in zip(result,candidate['hashes']))
                    arrays['mask'][i,j,selected]=True;arrays['valid'][i,j]=True;arrays['sources'][i,j]=candidate['sources']
                    arrays['outcome'][i,j]=result;arrays['cost_ms'][i,j]=cost
                    stats['candidates']+=1;stats['trials']+=8;stats['successful']+=int((result>0).sum())
                    stats['positive']+=int(((result>0)&(result<entry['current_T'])).sum())
                    stats['failed_extract']+=int((result==-1).sum());stats['failed_insert']+=int((result==-2).sum())
                assert sum(bool(c['sources']&4) for c in candidates)==1 and sum(bool(c['sources']&8) for c in candidates)==1
                if entry['audit_plan'] is not None:
                    assert not any(replay(text,{'current':entry['audit_plan']})['remaining'].values());stats['independent_replays']+=1
                stats['repeat_checked']|=entry['check_passed'] is True
        assert seen==set(range(count))
        if item['index']<4:assert stats['repeat_checked']
        (folder/'raw.jsonl').unlink()
    hashes={}
    for name,array in arrays.items():
        path=folder/f'{name}.npy';np.save(path,array);hashes[path.name]=sha(path)
    result={'input':item,'binary_sha256':digest,'arrays':hashes,'statistics':stats,'seconds':time.monotonic()-started,
            'encoding_source':str(ENCODING/'cases'/f"{item['index']:06d}"),'teacher_sources':paths,'completed_at':now()}
    save(folder/'complete.json',result);return result


def collect(run,binary,workers,limit):
    run.mkdir(parents=True,exist_ok=True);(run/'cases').mkdir(exist_ok=True)
    digest=sha(binary);config=run/'collection_config.json'
    if config.exists():assert json.loads(config.read_text())['binary_sha256']==digest
    else:
        save(config,{'binary_sha256':digest,'workers':workers,'source':str(SOURCE),'encoding':str(ENCODING),
                     'source_dataset_sha256':sha(ENCODING/'dataset.json'),'created_at':now()})
        shutil.copy2(binary,run/'collector_frozen')
    manifest=json.loads((SOURCE/'input_manifest.json').read_text());selected=manifest[:limit]
    started=time.monotonic();results=[]
    with ProcessPoolExecutor(max_workers=workers) as pool:
        futures=[pool.submit(collect_case,(str(run),item,str(binary),digest)) for item in selected]
        for future in as_completed(futures):
            results.append(future.result())
            status(run,'collecting',completed_inputs=len(results),total_inputs=len(selected),elapsed_seconds=time.monotonic()-started)
    results.sort(key=lambda row:row['input']['index'])
    stats={key:sum(r['statistics'][key] for r in results) for key in ('groups','candidates','trials','successful','positive','failed_extract','failed_insert','independent_replays')}
    result={'cases':results,'statistics':stats,'wall_seconds':time.monotonic()-started,'cpu_task_seconds':sum(r['seconds'] for r in results),
            'source_dataset_sha256':sha(ENCODING/'dataset.json'),'completed_at':now()}
    save(run/('dataset.json' if limit>=512 else 'pilot.json'),result)
    status(run,'data_ready' if limit>=512 else 'pilot_completed',**stats,wall_seconds=result['wall_seconds'])
    print(json.dumps({k:v for k,v in result.items() if k!='cases'}),flush=True)


class Dataset:
    def __init__(self,run):
        self.run=run;self.description=json.loads((run/'dataset.json').read_text())
        original=json.loads((ENCODING/'dataset.json').read_text())
        self.mean=np.array(original['mean'],np.float32);self.scale=np.array(original['scale'],np.float32)
        self.arrays={};self.rows=[];self.splits={'train':[],'validation':[]};self.input_counts={'train':0,'validation':0}
        for case in self.description['cases']:
            item=case['input'];index=item['index'];count=case['statistics']['groups']
            if not count:continue
            self.input_counts[item['role']]+=1;folder=run/'cases'/f'{index:06d}'
            values={name:np.load(folder/f'{name}.npy',mmap_mode='r') for name in ('mask','valid','sources','outcome','cost_ms','current_T')}
            values.update({name:np.load(ENCODING/'cases'/f'{index:06d}'/f'{name}.npy',mmap_mode='r') for name in ('nodes','edges')})
            self.arrays[index]=values
            for row in range(count):
                self.splits[item['role']].append(len(self.rows));self.rows.append((index,row,item['M'],count))
    def batch(self,indices):
        rows=[self.rows[i] for i in indices];B=len(rows);M=max(r[2] for r in rows)
        C=max(int(self.arrays[index]['valid'][row].sum()) for index,row,_,_ in rows)
        result={'nodes':np.zeros((B,M,24),np.float32),'edges':np.zeros((B,5,M,M),np.float32),'valid':np.zeros((B,M),bool),
                'mask':np.zeros((B,C,M),np.float32),'candidate_valid':np.zeros((B,C),bool),'sources':np.zeros((B,C),np.uint8),
                'gains':np.zeros((B,C,8),np.float32),'failed':np.zeros((B,C,8),np.float32),'cost_ms':np.zeros((B,C,8),np.float32),
                'weight':np.zeros(B,np.float32),'cases':np.zeros(B,np.int64)}
        for b,(index,row,m,count) in enumerate(rows):
            a=self.arrays[index];c=int(a['valid'][row].sum())
            result['nodes'][b,:m]=(a['nodes'][row]-self.mean)/self.scale;result['edges'][b,:,:m,:m]=a['edges'][row];result['valid'][b,:m]=True
            result['mask'][b,:c,:m]=a['mask'][row,:c];result['candidate_valid'][b,:c]=True
            result['sources'][b,:c]=a['sources'][row,:c];outcome=a['outcome'][row,:c]
            result['gains'][b,:c]=np.where(outcome>0,np.maximum(0,a['current_T'][row]-outcome),0)
            result['failed'][b,:c]=(outcome<0);result['cost_ms'][b,:c]=a['cost_ms'][row,:c]
            result['weight'][b]=1/count;result['cases'][b]=index
        return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--workers',type=int,default=30);p.add_argument('--limit',type=int,default=512);a=p.parse_args()
    collect(a.run.resolve(),a.binary.resolve(),a.workers,a.limit)
