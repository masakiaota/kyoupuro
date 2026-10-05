#!/usr/bin/env python3
"""入力ごとに有界な共同探索を実行し、確認済みの完成費用を保存する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT/'results/nn_rank/v116/20261004_finite_value_studio'
BC = ROOT/'results/nn_rank/v090/20261003_scaling_studio'
PYTHON = ROOT/'.venv-nn-v077/bin/python'


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix+'.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')
    temporary.replace(path)


def load(path):
    return json.loads(Path(path).read_text())


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1024*1024), b''):
            digest.update(block)
    return digest.hexdigest()


def selected():
    items = load(BC/'input_manifest.json')
    available = sorted((x for x in items if x['role'] == 'train'), key=lambda x:x['index'])
    assert len(available) == 4096
    train = [available[i] for i in np.linspace(0,4095,1024,dtype=int)]
    used = {x['index'] for x in train}
    remaining = [x for x in available if x['index'] not in used]
    dev = [remaining[i] for i in np.linspace(0,len(remaining)-1,64,dtype=int)]
    excluded = {x['sha256'] for x in items if x['role'] != 'train'}
    for name in ['in','validation1']:
        excluded.update(sha(x) for x in (ROOT/'tools'/name).glob('*.txt'))
    assert not any(x['sha256'] in excluded for x in train+dev), 'training input overlaps evaluation'
    result = [dict(x, guidance_role=role) for role, group in [('train',train),('development',dev)] for x in group]
    assert len({x['sha256'] for x in result}) == len(result)
    return result


def identity(root):
    return dict(parent_sha256=sha(ROOT/'src/bin/v111_weight_portfolio.cpp'),
                collector_sha256=sha(root/'frozen/collector'), baseline_binary_sha256=sha(root/'frozen/baseline'),
                width=96,node_limit=1536,window_seconds=2,windows_per_source=8,
                states_limit=32768,edges_limit=131072,samples_per_window=256)


def collect_case(root, item, frozen):
    directory = root/'data/cases'/f"{item['index']:06d}"
    directory.mkdir(parents=True, exist_ok=True)
    marker = directory/'complete.json'
    if marker.exists():
        result = load(marker)
        assert result['identity'] == frozen and result['input'] == item
        for name, digest in result['sha256'].items():
            assert sha(directory/name) == digest
        return result
    begin = time.monotonic()
    inp = BC/item['path']
    strong = BC/'cases'/f"{item['index']:06d}"/'best.txt'
    assert sha(inp) == item['sha256']
    assert sha(strong) == load(strong.parent/'complete.json')['best_sha256']
    plan = directory/'v111.txt'
    if not plan.exists():
        process = subprocess.run([root/'frozen/baseline'],input=inp.read_bytes(),capture_output=True,timeout=30,cwd=ROOT)
        (directory/'v111.log').write_bytes(process.stderr)
        if process.returncode:
            raise RuntimeError(f'baseline failed for {item["index"]}: {process.returncode}')
        temporary = plan.with_suffix('.tmp')
        temporary.write_bytes(process.stdout)
        temporary.replace(plan)
    process = subprocess.run([root/'frozen/collector','collect',inp,strong,plan,directory],
                             capture_output=True,text=True,timeout=300,cwd=ROOT)
    (directory/'collector.log').write_text(process.stderr)
    if process.returncode:
        raise RuntimeError(f'collector failed for {item["index"]}: {process.returncode}; {process.stderr[-2000:]}')
    statistics = json.loads(process.stdout)
    samples = np.fromfile(directory/'samples.raw',dtype='<f4')
    assert samples.size == statistics['samples']*51
    samples = samples.reshape(-1,51)
    assert np.isfinite(samples).all() and (samples[:,48] >= samples[:,0]).all()
    assert (samples[:,48] >= 0).all() and (samples[:,49] >= 0).all()
    assert (samples[:,50] >= 0).all() and (samples[:,50] < statistics['windows']).all()
    result = dict(input=item,identity=frozen,statistics=statistics,seconds=time.monotonic()-begin,
                  strong_sha256=sha(strong),sha256={name:sha(directory/name) for name in ['samples.raw','problems.txt','windows.jsonl','v111.txt']},
                  completed_at=now())
    save(marker,result)
    return result


def collect(root, pilot=False, workers=12):
    root = Path(root)
    if not pilot:
        assert load(root/'data/pilot.json')['gate_passed'], 'registered teacher gate has not passed'
    cases = selected()
    frozen = identity(root)
    manifest = root/'input_manifest.json'
    if manifest.exists():
        assert load(manifest) == cases
    else:
        save(manifest,cases)
    path = root/'data/identity.json'
    if path.exists():
        assert load(path) == frozen
    else:
        save(path,frozen)
    tasks = cases[:32] if pilot else cases
    started = time.monotonic()
    results = []
    with ThreadPoolExecutor(max_workers=workers) as pool:
        pending = {pool.submit(collect_case,root,item,frozen):item for item in tasks}
        for future in as_completed(pending):
            results.append(future.result())
            elapsed = time.monotonic()-started
            save(root/'data/status.json',dict(stage='pilot' if pilot else 'collection',completed=len(results),total=len(tasks),
                                             seconds=elapsed,remaining_seconds=elapsed*(len(tasks)-len(results))/len(results),updated_at=now()))
    results.sort(key=lambda x:x['input']['index'])
    statistics = {k:sum(x['statistics'][k] for x in results) for k in ['windows','samples','improved_windows','beats_baseline','independent_replays','representable_moves']}
    statistics['improved_inputs'] = sum(x['statistics']['improved_windows']>0 for x in results)
    gate = statistics['improved_inputs']>=2 and statistics['improved_windows']>=4 and statistics['samples']>=2048 and statistics['beats_baseline']>=1
    info = dict(cases=len(results),statistics=statistics,gate_passed=gate,seconds=time.monotonic()-started,identity=frozen,completed_at=now())
    save(root/'data'/('pilot.json' if pilot else 'collection.json'),info)
    if not pilot:
        consolidate(root,cases)
    return info


def consolidate(root,cases):
    paths = [(item,root/'data/cases'/f"{item['index']:06d}") for item in cases]
    descriptions=[]
    for role in ['train','development']:
        records = [(x,p,load(p/'complete.json')) for x,p in paths if x['guidance_role']==role]
        count = sum(r['statistics']['samples'] for _,_,r in records)
        array = np.lib.format.open_memmap(root/'data'/f'{role}.partial.npy',mode='w+',dtype=np.float32,shape=(count,51))
        offset=0
        for case_no,(item,path,record) in enumerate(records):
            values=np.fromfile(path/'samples.raw',dtype='<f4').reshape(-1,51)
            values[:,50]+=case_no*16
            array[offset:offset+len(values)]=values
            descriptions.append(dict(index=item['index'],role=role,start=offset,rows=len(values)))
            offset+=len(values)
        assert offset==count
        array.flush();del array
        (root/'data'/f'{role}.partial.npy').replace(root/'data'/f'{role}.npy')
    save(root/'data/dataset.json',dict(columns=51,features=48,records=descriptions,
                                      arrays={role:sha(root/'data'/f'{role}.npy') for role in ['train','development']},created_at=now()))


if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--run',type=Path,default=RUN)
    parser.add_argument('--pilot',action='store_true')
    parser.add_argument('--workers',type=int,default=12)
    args=parser.parse_args()
    print(json.dumps(collect(args.run,args.pilot,args.workers),ensure_ascii=False,indent=2))
