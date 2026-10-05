#!/usr/bin/env python3
"""選別逆教師の対照実験を、v092の固定終了重みから順に実行する。"""
import argparse
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v096_selected import ROOT,RUN,PARENT,configuration
from v091_env import load
from v090_data import save,sha,now,status


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True);lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    dataset=load(root/'teachers/dataset.json');assert dataset['training_eligible']
    assert load(PARENT/'pipeline/exit.json')['exit_code']==0
    assert load(root/'mechanism/result.json')['passed']
    from datetime import datetime
    if not (pipe/'config.json').exists():assert datetime.now().astimezone()<datetime.fromisoformat('2026-10-04T16:00:00+09:00')
    paths=[ROOT/'adhoc/scripts'/name for name in ('v096_selected.py','train_v096_selected.py','check_v096_model.py','run_v096_evaluation.py','run_v096_pipeline.py',
        'v092_stream.py','v091_env.py','train_v091_ppo.py','v090_data.py','train_v090_board.py','build_v090_board.py','v090_inference.cpp.txt',
        'v090_search.cpp.txt','v089_core.cpp.txt','v089_data.py','train_v089_board.py','check_v089_board.py','run_v089_evaluation.py',
        'train_v077_rank.py','train_v080_scaling.py','memory_guard.py')]
    paths += [ROOT/'adhoc/bin/v091_environment.cpp',ROOT/'scripts/build_solver.sh',ROOT/'scripts/eval.py']
    identity=dict(configs={v:configuration(v) for v in ('control','mixed')},source_sha256={str(p.relative_to(ROOT)):sha(p) for p in paths},
                  parent_checkpoint_sha256=sha(PARENT/'training/latest.pt'),reverse_dataset_sha256=sha(root/'teachers/dataset.json'))
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for path in paths+[ROOT/'notes/experiments/v096.md']:
            dest=pipe/'frozen'/path.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dest)
    started=time.monotonic()
    for variant in ('control','mixed'):
        where=root/variant;initial=where/'initial';initial.mkdir(parents=True,exist_ok=True)
        checkpoint=initial/'checkpoint.pt'
        if not checkpoint.exists():shutil.copy2(PARENT/'training/latest.pt',checkpoint)
        assert sha(checkpoint)==identity['parent_checkpoint_sha256']
        for stage,script,args,marker in (
            ('train','train_v096_selected.py',['--teachers',str(root)],where/'training/result.json'),
            ('numerical','check_v096_model.py',['--phase',variant],where/'numerical/result.json'),
            ('evaluate','run_v096_evaluation.py',['--phase',variant],where/'evaluation/comparison.json')):
            for path,fingerprint in identity['source_sha256'].items():assert sha(ROOT/path)==fingerprint,f'code changed: {path}'
            if marker.exists():continue
            status(pipe,variant+'_'+stage,pid=os.getpid(),marker=str(marker))
            with (pipe/f'{variant}_{stage}.log').open('a') as stream:
                process=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script,'--run',where]+args,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
            save(pipe/f'{variant}_{stage}_exit.json',dict(exit_code=process.returncode,finished_at=now()))
            assert process.returncode==0 and marker.exists(),f'{variant} {stage} failed'
    report=load(root/'mixed/evaluation/comparison.json');save(root/'comparison.json',report)
    status(pipe,'completed',next_stage='record v096; run the preregistered v097 short BC comparison before final candidate selection')
    save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args();root=a.run.resolve()
    try:execute(root)
    except BlockingIOError:print('pipeline already running',file=sys.stderr);sys.exit(76)
    except BaseException as e:
        save(root/'pipeline/exit.json',dict(exit_code=1,error=repr(e),finished_at=now()));traceback.print_exc();sys.exit(1)
