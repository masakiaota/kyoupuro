#!/usr/bin/env python3
"""新規地形の逆生成状態から、固定方策より短い既知の解答だけを選ぶ。"""
import argparse
from concurrent.futures import ThreadPoolExecutor,as_completed
import fcntl
import json
from pathlib import Path
import shutil
import subprocess
import time
import traceback
import numpy as np
from v089_data import ROOT,Geometry,remaining,save,sha,now,status
from v091_env import load
from v092_stream import excluded_inputs,normalized,digest,describe
from run_v093_reverse import one

RUN=ROOT/'results/nn_rank/v096/20261003_selected_studio'
SOURCE=ROOT/'results/nn_rank/v093/20261003_reverse_studio'
PPO=ROOT/'results/nn_rank/v092/20261003_fresh_studio'
CONFIG=dict(count=2048,depth=64,workers=8,seconds=2700,seed_base=960000000000,minimum_geometries=512,minimum_states=4096)


def prepare_inputs(root):
    d=root/'teachers';d.mkdir(parents=True,exist_ok=True)
    if (d/'inputs.json').exists():return load(d/'inputs.json')
    seeds=[CONFIG['seed_base']+i for i in range(CONFIG['count'])];p=d/'seeds.txt';p.write_text(''.join(f'{s}\n' for s in seeds))
    subprocess.run([root/'generator',p,'--dir',d/'inputs'],check=True,stdout=subprocess.DEVNULL)
    blocked=excluded_inputs();seen=set();rows=[]
    for i,seed in enumerate(seeds):
        path=d/'inputs'/f'{i:04d}.txt';text=normalized(path.read_text());fingerprint=digest(text)
        assert fingerprint not in blocked and fingerprint not in seen;seen.add(fingerprint)
        _,stats=describe(text);rows.append(dict(index=i,seed=seed,path=str(path.relative_to(root)),sha256=sha(path),**stats))
    save(d/'inputs.json',rows);return rows


def action_line(code):
    p=code&511;return f"{p//20} {p%20} {(code>>9)&7} {'UDLR'[(code>>12)&3]} {1+((code>>14)&7)}"


def select_one(root,item):
    d=root/'teachers/cases'/f"{item['index']:04d}";marker=d/'selected.json'
    if marker.exists():return load(marker)
    teacher=one(root,'teachers',item,CONFIG['depth']);T=teacher['depth']
    eval_done=d/'policy_evaluation.json'
    if not eval_done.exists():
        started=d/'policy_started.json'
        assert not started.exists(),f'partial policy output needs inspection: {d}'
        save(started,dict(started_at=now()))
        subprocess.run([root/'select_teachers',root/item['path'],d,d],check=True,capture_output=True,text=True,timeout=180.)
        save(eval_done,dict(completed_at=now(),cases=T))
    values=np.loadtxt(d/'policy_rows.txt',ndmin=2);assert len(values)==T
    actions=np.fromfile(d/'policy_actions.raw',dtype='<u4');offsets=np.fromfile(d/'policy_offsets.raw',dtype='<u8')
    states=np.fromfile(d/'states.raw',dtype='<u4').reshape(T+1,400);assert len(offsets)==T+1 and offsets[-1]==len(actions)
    geo=Geometry(root/item['path']);selected=[];wins=[];failures=0
    for frame,row in enumerate(values):
        idx,steps,E,deadline,seconds=row;assert int(idx)==frame
        state=states[frame].copy();path=actions[offsets[frame]:offsets[frame+1]];assert len(path)==int(steps)
        for code in path:geo.apply(state,action_line(int(code)))
        assert remaining(state)==int(E)
        teacher_T=T-frame
        if E>0 or teacher_T<steps:
            selected.append(dict(frame=frame,teacher_T=teacher_T,policy_T=int(steps),policy_E=int(E),deadline=bool(deadline)))
            if E>0:failures+=1
            else:wins.append(int(steps)-teacher_T)
    result=dict(teacher=teacher,selected=selected,evaluated_states=T,selected_states=len(selected),policy_failed=failures,
                common_complete_gain_sum=sum(wins),common_complete_gain_max=max(wins,default=0),all_policy_outputs_legal=True)
    save(marker,result);return result


def execute(root):
    root.mkdir(parents=True,exist_ok=True);d=root/'teachers';d.mkdir(exist_ok=True)
    lock=(d/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (d/'dataset.json').exists():return load(d/'dataset.json')
    paths=[Path(__file__).resolve(),ROOT/'adhoc/bin/select_v096_teachers.cpp',ROOT/'adhoc/scripts/run_v093_reverse.py',ROOT/'adhoc/bin/generate_v093_reverse.cpp',ROOT/'adhoc/bin/v092_start.cpp']
    identity=dict(config=CONFIG,sources={str(p.relative_to(ROOT)):sha(p) for p in paths},reference_model_sha256=sha(PPO/'initial/model.json'))
    if (d/'config.json').exists():assert load(d/'config.json')==identity
    else:
        save(d/'config.json',identity)
        for p in paths+[ROOT/'notes/experiments/v096.md']:
            dest=d/'frozen'/p.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest)
        for a,b in ((SOURCE/'generator',root/'generator'),(SOURCE/'reverse_generator',root/'reverse_generator'),(ROOT/'target/release/select_v096_teachers',root/'select_teachers')):shutil.copy2(a,b)
    inputs=prepare_inputs(root);start_path=d/'budget.json'
    budget=load(start_path) if start_path.exists() else dict(used_seconds=0.)
    started=time.monotonic();finished=[]
    def task(item):
        marker=d/'cases'/f"{item['index']:04d}"/'selected.json'
        if marker.exists():return load(marker)
        if time.monotonic()-started+budget['used_seconds']>=CONFIG['seconds']:return None
        return select_one(root,item)
    status(d,'generating_and_selecting',total=len(inputs),workers=CONFIG['workers'])
    try:
        with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:
            futures=[pool.submit(task,item) for item in inputs]
            for future in as_completed(futures):
                row=future.result()
                if row is not None:finished.append(row)
                if len(finished) and len(finished)%64==0:
                    elapsed=time.monotonic()-started
                    status(d,'generating_and_selecting',completed=len(finished),total=len(inputs),seconds=elapsed+budget['used_seconds'],
                           selected_states=sum(r['selected_states'] for r in finished),remaining_seconds=min(CONFIG['seconds']-elapsed-budget['used_seconds'],elapsed*(len(inputs)-len(finished))/len(finished)))
    finally:
        budget['used_seconds']+=time.monotonic()-started;save(start_path,budget)
    finished.sort(key=lambda r:r['teacher']['index']);count=sum(r['selected_states'] for r in finished)
    shapes={'states':(np.uint32,(count,400)),'actions':(np.uint32,(count,)),'case_ids':(np.int32,(count,)),'known_steps':(np.float32,(count,))}
    arrays={k:np.lib.format.open_memmap(d/(k+'.npy'),mode='w+',dtype=dt,shape=shape) for k,(dt,shape) in shapes.items()}
    cursor=0;manifest=[]
    for r in finished:
        teacher=r['teacher'];cid=teacher['index'];where=d/'cases'/f'{cid:04d}';T=teacher['depth']
        states=np.fromfile(where/'states.raw',dtype='<u4').reshape(T+1,400);actions=np.fromfile(where/'actions.raw',dtype='<u4')
        chosen=[s['frame'] for s in r['selected']];n=len(chosen)
        arrays['states'][cursor:cursor+n]=states[chosen];arrays['actions'][cursor:cursor+n]=actions[chosen]
        arrays['case_ids'][cursor:cursor+n]=cid;arrays['known_steps'][cursor:cursor+n]=T-np.array(chosen)
        manifest.append(dict(teacher,selected_frames=n,frame_start=cursor));cursor+=n
    assert cursor==count
    for a in arrays.values():a.flush()
    geometries=sum(r['selected_states']>0 for r in finished);total=sum(r['evaluated_states'] for r in finished)
    result=dict(cases=manifest,generated_geometries=len(finished),selected_geometries=geometries,evaluated_states=total,selected_states=count,
                selection_rate=count/total if total else 0.,all_legal=True,all_teachers_complete=True,policy_failed=sum(r['policy_failed'] for r in finished),
                gain_sum=sum(r['common_complete_gain_sum'] for r in finished),gain_max=max((r['common_complete_gain_max'] for r in finished),default=0),
                training_eligible=geometries>=CONFIG['minimum_geometries'] and count>=CONFIG['minimum_states'],
                array_sha256={k:sha(d/(k+'.npy')) for k in arrays},seconds=budget['used_seconds'],completed_at=now())
    save(d/'dataset.json',result);status(d,'completed',**{k:v for k,v in result.items() if k not in ('cases','array_sha256')});save(d/'exit.json',dict(exit_code=0,finished_at=now()))
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args()
    try:execute(a.run.resolve())
    except BaseException as error:
        save(a.run/'teachers/exit.json',dict(exit_code=1,error=repr(error),finished_at=now()));traceback.print_exc();raise
