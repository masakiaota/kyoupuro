#!/usr/bin/env python3
"""保存済み逆生成列の終盤を深さ別に比較する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import fcntl
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time
import traceback
import numpy as np
from v089_data import ROOT,Geometry,remaining,save,sha,now,status
from v091_env import load
from check_v089_board import trace
from run_v093_reverse import one,prepare_inputs

RUN=ROOT/'results/nn_rank/v095/20261003_endgame_studio'
SOURCE=ROOT/'results/nn_rank/v093/20261003_reverse_studio'
PPO=ROOT/'results/nn_rank/v092/20261003_fresh_studio'
DEPTHS=[1,2,4,8,16,32,64]


def evaluate(root,item,depth):
    directory=root/'comparison'/f"{item['index']:04d}_{depth:03d}";directory.mkdir(parents=True,exist_ok=True)
    marker=directory/'result.json'
    if marker.exists():return load(marker)
    source=SOURCE/'pilot/cases'/f"{item['index']:04d}";T=item['depth'];assert depth<=T
    all_states=np.fromfile(source/'states.raw',dtype='<u4').reshape(T+1,400);state=all_states[T-depth].copy()
    input_path=SOURCE/item['path'];geo=Geometry(input_path);copy=state.copy()
    teacher=(source/'solution.txt').read_text().splitlines()[T-depth:]
    assert len(teacher)==depth
    for t,line in enumerate(teacher):geo.apply(copy,line);assert np.array_equal(copy,all_states[T-depth+t+1])
    assert remaining(copy)==0
    if depth==T:
        old=load(source/'policy.json');E=old['E'];steps=old['T'];elapsed=old['elapsed_seconds'];reused=True
    else:
        marker_start=directory/'started.json';assert not marker_start.exists(),'partial evaluation needs inspection'
        snapshot=directory/'snapshot.txt';snapshot.write_text(' '.join(map(str,state))+'\n')
        binary=PPO/'numerical/start/checker_local';save(marker_start,dict(binary_sha256=sha(binary),started_at=now()))
        tick=time.monotonic();proc=subprocess.run([binary,snapshot],input=input_path.read_text(),text=True,capture_output=True,check=True,timeout=10.)
        elapsed=time.monotonic()-tick;(directory/'policy.txt').write_text(proc.stdout);(directory/'policy.err').write_text(proc.stderr)
        copy=state.copy();lines=[line for line in proc.stdout.splitlines() if line.strip()]
        for line in lines:geo.apply(copy,line)
        E=remaining(copy);steps=len(lines);counts=trace(proc.stderr);assert counts['E']==E and counts['T']==steps;reused=False
    terrain='\n'.join(''.join('.' if c.islower() else c for c in line) for line in geo.C)
    result=dict(index=item['index'],depth=depth,initial_E=remaining(state),teacher_T=depth,policy_T=steps,policy_E=E,
                teacher_minus_policy_T=depth-steps if E==0 else None,elapsed_seconds=elapsed,reused=reused,all_legal=True,
                state_sha256=hashlib.sha256(terrain.encode()+state.tobytes()).hexdigest())
    save(marker,result);return result


def summarize(rows):
    common=[r for r in rows if r['policy_E']==0];diff=[r['teacher_minus_policy_T'] for r in common]
    assert len({r['state_sha256'] for r in rows})==len(rows)
    return dict(cases=len(rows),policy_completed=len(common),mean_teacher_minus_policy_T=float(np.mean(diff)) if diff else None,
                strict_wins=sum(d<0 for d in diff),no_longer=sum(d<=0 for d in diff),mean_initial_E=float(np.mean([r['initial_E'] for r in rows])))


def execute(root):
    root.mkdir(parents=True,exist_ok=True);lock=(root/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    paths=[Path(__file__).resolve(),ROOT/'adhoc/scripts/run_v093_reverse.py',ROOT/'adhoc/bin/generate_v093_reverse.cpp']
    identity=dict(depths=DEPTHS,sources={str(p.relative_to(ROOT)):sha(p) for p in paths},
                  parent_quality_sha256=sha(SOURCE/'pilot/result.json'),model_sha256=sha(PPO/'initial/model.json'))
    if (root/'config.json').exists():assert load(root/'config.json')==identity
    else:
        save(root/'config.json',identity)
        for p in paths+[ROOT/'notes/experiments/v095.md']:
            dest=root/'frozen'/p.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest)
        for old,new in ((SOURCE/'generator',root/'generator'),(SOURCE/'reverse_generator',root/'reverse_generator')):shutil.copy2(old,new)
    marker=root/'comparison.json'
    if not marker.exists():
        cases=load(SOURCE/'pilot/cases.json');tasks=[(c,d) for c in cases for d in DEPTHS if d<=c['depth']]
        status(root,'depth_comparison',cases=len(tasks))
        with ThreadPoolExecutor(max_workers=8) as pool:rows=list(pool.map(lambda task:evaluate(root,*task),tasks))
        individual={str(d):summarize([r for r in rows if r['depth']==d]) for d in DEPTHS}
        cumulative={str(d):summarize([r for r in rows if r['depth']<=d]) for d in DEPTHS}
        eligible=[]
        for d in DEPTHS:
            v=cumulative[str(d)];n=v['policy_completed']
            if n and v['mean_teacher_minus_policy_T']<0 and v['no_longer']>=.5*n and v['strict_wins']>=.1*n:eligible.append(d)
        result=dict(by_depth=individual,cumulative=cumulative,selected_depth=max(eligible) if eligible else None,rows=rows,completed_at=now())
        save(marker,result);print(json.dumps({k:v for k,v in result.items() if k!='rows'}),flush=True)
    result=load(marker);depth=result['selected_depth']
    if depth is not None and not (root/'bulk/result.json').exists():
        status(root,'teacher_generation',depth=depth,count=1024);inputs=prepare_inputs(root,'bulk',1024,20000);started=time.monotonic()
        def prepare(item):
            if time.monotonic()-started>1800:return None
            return one(root,'bulk',item,depth)
        with ThreadPoolExecutor(max_workers=8) as pool:teachers=[r for r in pool.map(prepare,inputs) if r is not None]
        assert len({r['input_state_sha256'] for r in teachers})==len(teachers)
        save(root/'bulk/result.json',dict(cases=teachers,depth=depth,frames=sum(r['depth'] for r in teachers),all_legal=True,
             all_complete=True,seconds=time.monotonic()-started,completed_at=now()))
    status(root,'completed',selected_depth=depth);save(root/'exit.json',dict(exit_code=0,finished_at=now()))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args()
    try:execute(a.run.resolve())
    except BaseException as error:
        save(a.run/'exit.json',dict(exit_code=1,error=repr(error),finished_at=now()));traceback.print_exc();raise
