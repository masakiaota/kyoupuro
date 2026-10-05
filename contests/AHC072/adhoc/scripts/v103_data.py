#!/usr/bin/env python3
"""固定NNの完成列を短縮し、区間境界と全列を独立再生して教師にする。"""
import argparse
from concurrent.futures import ProcessPoolExecutor, as_completed
import difflib
import json
from pathlib import Path
import shutil
import subprocess
import time

import numpy as np
from v089_data import ROOT, Geometry, remaining, save, sha, status, now
from v091_env import load
from v092_stream import excluded_inputs, normalized, digest, describe

PARENT = ROOT / 'results/nn_rank/v102/20261004_bc_ablation_studio/without_bc'
INITIAL = PARENT / 'training/latest.pt'
INITIAL_SHA = 'c4fc2cd727e65382ef6f5f14d7dd755230b0fe68de1414a5ba48b09616f285d9'
BASE_SOURCE = ROOT / 'adhoc/bin/v102_without_bc.cpp'


def replay(path, lines):
    geo = Geometry(path); state = geo.initial.copy(); states = [state.copy()]; actions = []
    for line in lines:
        actions.append(geo.apply(state, line)); states.append(state.copy())
    assert remaining(state) == 0, 'teacher must complete every return'
    return np.asarray(states, np.uint32), np.asarray(actions, np.uint32)


def alignment(before, after, old_actions, new_actions):
    """同じ全盤面の境界を結び、変更区間を取り出す。最短性は主張しない。"""
    assert np.array_equal(before[0], after[0]) and np.array_equal(before[-1], after[-1])
    a = [s.tobytes() for s in before]; b = [s.tobytes() for s in after]
    matches = difflib.SequenceMatcher(a=a, b=b, autojunk=False).get_matching_blocks()
    pairs = [(0, 0)]
    for block in matches:
        for t in range(block.size):
            i, j = block.a+t, block.b+t
            if i > pairs[-1][0] and j > pairs[-1][1]: pairs.append((i, j))
    end = (len(old_actions), len(new_actions))
    if pairs[-1] != end: pairs.append(end)
    old_mask = np.zeros(len(old_actions), bool); new_mask = np.zeros(len(new_actions), bool); intervals = []
    for (i, j), (ni, nj) in zip(pairs, pairs[1:]):
        assert a[i] == b[j] and a[ni] == b[nj]
        if np.array_equal(old_actions[i:ni], new_actions[j:nj]): continue
        old_mask[i:ni] = True; new_mask[j:nj] = True
        intervals.append(dict(original=[i, ni], shortened=[j, nj], saved=ni-i-(nj-j),
                              entry_sha256=digest(a[i].hex()), exit_sha256=digest(a[ni].hex())))
    assert sum(r['saved'] for r in intervals) == len(old_actions)-len(new_actions)
    return intervals, old_mask, new_mask


def compile_helper(name, destination):
    if destination.exists(): return
    subprocess.run([ROOT/'scripts/build_solver.sh', name], check=True, stdout=subprocess.PIPE)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT/'target/release'/name, destination)


def generate(data, count, seed_base):
    marker = data/'input_manifest.json'
    if marker.exists():
        rows = load(marker); assert len(rows) == count
        for row in rows: assert sha(data/row['path']) == row['sha256']
        return rows
    blocked = excluded_inputs()
    diagnostic = data.parents[1]/'diagnostic/data/input_manifest.json'
    if data.parent.name == 'main' and diagnostic.exists():
        for row in load(diagnostic):blocked.add(digest(normalized((diagnostic.parent/row['path']).read_text())))
    save(data/'excluded_sha256.json', sorted(blocked))
    generator = ROOT/'tools/target/release/gen'; rows = []; seed = seed_base
    raw = data/'generated'; raw.mkdir(parents=True, exist_ok=True)
    # 生成は有限個であり、保存した入力だけで全工程を再開できる。
    while len(rows) < count:
        seeds = list(range(seed, seed+count-len(rows))); (data/'seeds.txt').write_text('\n'.join(map(str, seeds))+'\n')
        subprocess.run([generator, data/'seeds.txt', '--dir', raw], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        for offset, value in enumerate(seeds):
            text = normalized((raw/f'{offset:04d}.txt').read_text()); fingerprint = digest(text)
            if fingerprint in blocked: continue
            blocked.add(fingerprint); index = len(rows); path = data/'cases'/f'{index:06d}'/'input.txt'
            path.parent.mkdir(parents=True, exist_ok=True); path.write_text(text)
            _, properties = describe(text)
            rows.append(dict(index=index, seed=value, path=str(path.relative_to(data)), sha256=sha(path), **properties))
        seed += len(seeds)
    save(marker, rows); save(data/'generator.json', dict(binary_sha256=sha(generator), next_seed=seed, count=count))
    return rows


def improve_one(data_text, item, baseline_text, shortener_text):
    data=Path(data_text); path=data/item['path']; directory=path.parent; marker=directory/'result.json'
    if marker.exists(): return load(marker)
    assert sha(path)==item['sha256']; original=directory/'original.txt'
    if not original.exists():
        start=time.monotonic()
        proc=subprocess.run([baseline_text],input=path.read_text(),capture_output=True,text=True,check=True,timeout=30)
        original.write_text(proc.stdout);(directory/'original.err').write_text(proc.stderr)
        save(directory/'original_time.json',dict(seconds=time.monotonic()-start))
    original_lines=original.read_text().splitlines()
    before, original_actions=replay(path,original_lines)
    after_path=directory/'shortened.txt'; stage_dir=directory/'stages'
    if not after_path.exists():
        proc=subprocess.run([shortener_text,original,stage_dir],input=path.read_text(),capture_output=True,text=True,check=True,timeout=45)
        after_path.write_text(proc.stdout); save(directory/'shortener.json',json.loads(proc.stderr))
    mechanism=load(directory/'shortener.json'); records=[]
    previous, previous_actions=before,original_actions
    for stage in range(1,mechanism['stages']):
        lines=(stage_dir/f'{stage}.txt').read_text().splitlines(); current,actions=replay(path,lines)
        assert len(actions)<len(previous_actions)
        intervals,_,_=alignment(previous,current,previous_actions,actions)
        records.append(dict(stage=stage,intervals=intervals))
        previous,previous_actions=current,actions
    after,short_actions=replay(path,after_path.read_text().splitlines())
    assert np.array_equal(previous,after) and np.array_equal(previous_actions,short_actions)
    assert mechanism['original_T']==len(original_actions) and mechanism['T']==len(short_actions)
    intervals,old_mask,new_mask=alignment(before,after,original_actions,short_actions)
    gain=len(original_actions)-len(short_actions)
    assert gain>=0 and gain==mechanism['joint_saved']+mechanism['finite_saved']
    if gain: assert old_mask.any() and new_mask.any()
    for variant,states,actions,changed in (('original',before,original_actions,old_mask),('shortened',after,short_actions,new_mask)):
        target=directory/variant;target.mkdir(exist_ok=True)
        np.save(target/'states.npy',states[:-1]);np.save(target/'actions.npy',actions);np.save(target/'changed.npy',changed)
    save(directory/'intervals.json',dict(stages=records,final=intervals))
    result=dict(**item,original_T=len(original_actions),T=len(short_actions),saved=gain,eligible=gain>0,
                changed_original=int(old_mask.sum()),changed_shortened=int(new_mask.sum()),
                original_sha256=sha(original),shortened_sha256=sha(after_path),mechanism=mechanism,all_legal=True,E=0)
    save(marker,result);return result


def assemble(data, rows):
    selected=[r for r in rows if r['eligible']]
    for variant in ('original','shortened'):
        directory=data/variant;directory.mkdir(exist_ok=True);counts=[r['original_T'] if variant=='original' else r['T'] for r in selected]
        total=sum(counts);arrays={}
        for key,dtype,shape in [('states',np.uint32,(total,400)),('actions',np.uint32,(total,)),('changed',bool,(total,))]:
            arrays[key]=np.lib.format.open_memmap(directory/(key+'.npy'),mode='w+',dtype=dtype,shape=shape)
        cases=[];start=0
        for row,count in zip(selected,counts):
            case_dir=(data/row['path']).parent/variant
            for key,a in arrays.items():a[start:start+count]=np.load(case_dir/(key+'.npy'),mmap_mode='r')
            cases.append(dict(row,frame_start=start,frames=count));start+=count
        for a in arrays.values():a.flush()
        save(directory/'dataset.json',dict(variant=variant,cases=cases,frames=total,root=str(data),
                    array_sha256={key:sha(directory/(key+'.npy')) for key in arrays}))
    return selected


def prepare(root, phase):
    data=root/phase/'data'; data.mkdir(parents=True,exist_ok=True);marker=data/'result.json'
    if marker.exists(): return load(marker)
    count,seed=(32,1030000000000) if phase=='diagnostic' else (1024,1030000010000)
    assert sha(INITIAL)==INITIAL_SHA
    baseline=root/'binaries/baseline';shortener=root/'binaries/shortener'
    compile_helper(BASE_SOURCE.stem,baseline);compile_helper('v103_shorten',shortener)
    identity=dict(initial_sha256=INITIAL_SHA,baseline_source_sha256=sha(BASE_SOURCE),shortener_source_sha256=sha(ROOT/'adhoc/bin/v103_shorten.cpp'),
                  baseline_binary_sha256=sha(baseline),shortener_binary_sha256=sha(shortener),count=count,seed_base=seed)
    if (data/'config.json').exists():assert load(data/'config.json')==identity
    else:save(data/'config.json',identity)
    items=generate(data,count,seed);rows=[];started=time.monotonic()
    with ProcessPoolExecutor(max_workers=20) as pool:
        jobs=[pool.submit(improve_one,str(data),row,str(baseline),str(shortener)) for row in items]
        for future in as_completed(jobs):
            rows.append(future.result())
            if len(rows)%8==0:status(data,'generating_teachers',completed=len(rows),total=count,seconds=time.monotonic()-started)
    rows.sort(key=lambda r:r['index']);selected=assemble(data,rows)
    average=float(np.mean([r['saved'] for r in selected])) if selected else 0.
    passed=len(selected)>=16 and average>=2 if phase=='diagnostic' else len(selected)>=128
    result=dict(rows=rows,selected=len(selected),mean_saved_selected=average,quality_gate=bool(passed),
                all_legal=True,seconds=time.monotonic()-started,completed_at=now())
    save(marker,result);status(data,'completed',selected=len(selected),quality_gate=bool(passed),mean_saved_selected=average)
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);p.add_argument('--phase',choices=('diagnostic','main'),required=True)
    args=p.parse_args();prepare(args.run.resolve(),args.phase)
