#!/usr/bin/env python3
"""時間と共通更新数を固定した三段階方策の短時間比較。"""
import argparse
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v097_policy import ROOT,RUN,PARENT,CONFIG
from v091_env import load
from v090_data import save,sha,now,status


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True);lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    assert load(root/'mechanism/numerical/result.json')['passed']
    assert load(root/'mechanism/training_result.json')['passed']
    paths=[ROOT/'adhoc/scripts'/n for n in ('v097_policy.py','train_v097_policy.py','build_v097_policy.py','check_v097_policy.py','check_v097_flat.py','run_v097_evaluation.py','run_v097_pipeline.py',
        'v097_inference.cpp.txt','v097_search.cpp.txt','v092_stream.py','v091_env.py','v090_data.py','train_v090_board.py','build_v090_board.py',
        'v090_inference.cpp.txt','v090_search.cpp.txt','v089_core.cpp.txt','v089_data.py','train_v089_board.py','check_v089_board.py','run_v089_evaluation.py','train_v077_rank.py','train_v080_scaling.py','memory_guard.py')]
    paths += [ROOT/p for p in ('adhoc/bin/v097_environment.cpp','adhoc/bin/v091_environment.cpp','scripts/build_solver.sh','scripts/eval.py')]
    identity=dict(config=CONFIG,sources={str(p.relative_to(ROOT)):sha(p) for p in paths},initial_sha256=sha(PARENT/'initial/checkpoint.pt'))
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for p in paths+[ROOT/'notes/experiments/v097.md']:
            dest=pipe/'frozen'/p.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest)
    started=time.monotonic()
    def run(label,script,args,marker):
        for p,fingerprint in identity['sources'].items():assert sha(ROOT/p)==fingerprint,f'code changed: {p}'
        if marker.exists():return
        status(pipe,label,pid=os.getpid())
        with (pipe/(label+'.log')).open('a') as stream:
            process=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script,'--run',root]+args,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/(label+'_exit.json'),dict(exit_code=process.returncode,finished_at=now()))
        assert process.returncode==0 and marker.exists(),label+' failed'
    for variant in ('flat','factor'):run('train_'+variant,'train_v097_policy.py',['--variant',variant],root/variant/'result.json')
    step=min(load(root/v/'result.json')['last_shared_milestone'] for v in ('flat','factor'))
    if step==0:
        status(pipe,'insufficient_training',step=0);save(pipe/'exit.json',dict(exit_code=0,insufficient_training=True,finished_at=now()));return
    selection=dict(step=step,rule='largest common saved step; no rollout result used')
    if (root/'selection.json').exists():assert load(root/'selection.json')==selection
    else:save(root/'selection.json',selection)
    for variant in ('flat','factor'):
        original=root/variant/f'model_step{step:05d}.json';destination=root/variant/'model.json'
        if not destination.exists():shutil.copy2(original,destination)
        assert sha(original)==sha(destination)
    run('numerical_flat','check_v097_flat.py',[],root/'numerical/flat/result.json')
    run('numerical_factor','check_v097_policy.py',[],root/'numerical/factor/result.json')
    run('evaluate','run_v097_evaluation.py',[],root/'evaluation/comparison.json')
    status(pipe,'completed',seconds=time.monotonic()-started);save(pipe/'exit.json',dict(exit_code=0,finished_at=now(),seconds=time.monotonic()-started))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args();root=a.run.resolve()
    try:execute(root)
    except BlockingIOError:print('pipeline already running',file=sys.stderr);sys.exit(76)
    except BaseException as e:
        save(root/'pipeline/exit.json',dict(exit_code=1,error=repr(e),finished_at=now()));traceback.print_exc();sys.exit(1)
