#!/usr/bin/env python3
"""教師診断を通過した条件の機構確認、短時間学習、固定評価を再開可能に実行する。"""
import argparse
from datetime import datetime
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v089_data import ROOT,save,sha,status,now
from v091_env import load
from v112_learning import configuration,INITIAL_SHA


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True)
    lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code']==0:return
    assert sha(root/'initial/checkpoint.pt')==INITIAL_SHA
    assert load(root/'diagnostic/result.json')['promising_teacher']
    names=('run_v112_pipeline.py','train_v112_lns.py','v112_learning.py','assess_v112_lns.py',
           'run_v112_labels.py','v112_main.cpp.txt','v112_observe.cpp.txt','v111_portfolio.py',
           'v102_learning.py','v101_compute.py','v099_complete.py','v104_extended.py','v109_dp.py',
           'v092_stream.py','v091_env.py','v090_data.py','v089_data.py','v096_selected.py',
           'train_v091_ppo.py','train_v090_board.py','train_v089_board.py','train_v077_rank.py',
           'train_v080_scaling.py','build_v090_board.py','check_v089_board.py','check_v098_complete.py',
           'run_v089_evaluation.py','run_v105_hybrid.py','v089_core.cpp.txt','v090_inference.cpp.txt',
           'v090_search.cpp.txt','memory_guard.py')
    paths=[ROOT/'adhoc/scripts'/name for name in names]
    paths += [ROOT/'adhoc/bin/v091_environment.cpp',ROOT/'adhoc/bin/v112_lns_labels.cpp',
              ROOT/'src/bin/v401_incremental_cnn.cpp',ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh']
    control=ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'
    identity=dict(config=configuration(),initial_checkpoint_sha256=INITIAL_SHA,
                  diagnostic_sha256=sha(root/'diagnostic/result.json'),control_sha256=sha(control/'result.json'),
                  sources={str(p.relative_to(ROOT)):sha(p) for p in paths})
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for path in paths+[ROOT/'notes/experiments/v112.md']:
            target=pipe/'frozen'/path.relative_to(ROOT);target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,target)
    def stage(name,script,args,marker):
        if marker.exists():return
        for path,fingerprint in identity['sources'].items():assert sha(ROOT/path)==fingerprint,path
        status(pipe,name,pid=os.getpid(),marker=str(marker))
        with (pipe/(name+'.log')).open('a') as stream:
            process=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script]+list(map(str,args)),
                                   cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/(name+'_exit.json'),dict(exit_code=process.returncode,completed_at=now()))
        assert process.returncode==0 and marker.exists(),f'{name} failed; inspect saved log'
    started=time.monotonic()
    try:
        stage('mechanism','train_v112_lns.py',['--run',root,'--phase','mechanism','--gpu-state',root/'gpu_state.json'],root/'mechanism/result.json')
        assert load(root/'mechanism/result.json')['passed']
        training=root/'training'
        late=datetime.now().astimezone()>=datetime.fromisoformat('2026-10-04T17:10:00+09:00')
        if late and not (training/'latest.pt').exists():
            result=dict(trained=False,accepted=False,reason='registered start deadline exceeded',completed_at=now())
        else:
            stage('train','train_v112_lns.py',['--run',root,'--phase','train','--gpu-state',root/'gpu_state.json'],training/'result.json')
            stage('assessment','assess_v112_lns.py',['--run',root],root/'final/result.json')
            result=dict(trained=True,training=load(training/'result.json'),assessment=load(root/'assessment.json'),
                        final=load(root/'final/result.json'),completed_at=now())
        save(root/'result.json',result)
        save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,completed_at=now()))
        status(pipe,'completed',next_stage='verify, record, commit/push, stop heartbeat')
    except BaseException as error:
        save(pipe/'exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),completed_at=now()))
        status(pipe,'failed',error=repr(error));raise


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--run',type=Path,required=True)
    args=parser.parse_args();execute(args.run.resolve())
