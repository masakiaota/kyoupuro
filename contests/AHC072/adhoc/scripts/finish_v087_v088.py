#!/usr/bin/env python3
"""報酬学習、独立な600周延長、固定した探索評価を順番に完了させる。"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from v086_data import ROOT, save, sha, now

V087=ROOT/'results/nn_rank/v087/20261002_reward_studio'
V088=ROOT/'results/nn_rank/v088/20261002_long_studio'
PIPE=V088/'continuation'
PYTHON=ROOT/'.venv-nn-v077/bin/python'


def status(stage,**values):
    save(PIPE/'status.json',{'stage':stage,'updated_at':now(),**values})
    print(json.dumps({'stage':stage,**values}),flush=True)


def verify():
    for path,digest in json.loads((PIPE/'source_sha256.json').read_text()).items():
        if sha(ROOT/path)!=digest:raise RuntimeError(f'source changed: {path}')


def run(args,log,stage,allow_failure=False):
    verify()
    with log.open('w') as out:
        child=subprocess.Popen(list(map(str,args)),cwd=ROOT,stdout=out,stderr=subprocess.STDOUT)
        status(stage,pid=child.pid)
        code=child.wait()
    if code and not allow_failure:raise RuntimeError(f'{stage} exited {code}: {log}')
    return code


def main():
    verify();started=time.monotonic()
    assert json.loads((V087/'learning_check.json').read_text())['passed']
    assert not (V087/'guard_train').exists() and not (V088/'guard600').exists()
    run([PYTHON,ROOT/'adhoc/scripts/memory_guard.py','--log-dir',V087/'guard_train','--seconds','3600',
         '--gpu-state',V087/'gpu_state.json','--',PYTHON,ROOT/'adhoc/scripts/train_v087_value.py',
         '--run',V087,'--seconds','3540'],V087/'train.log','training_reward_model',allow_failure=True)
    previous=json.loads((V087/'guard_train/exit.json').read_text())
    save(PIPE/'v087_exit.json',previous)
    if previous.get('exit_code')!=0:
        # 実験は独立している。失敗は保持し、明示指示された学習期間の比較を続ける。
        status('v087_failed_continuing_independent_v088',exit=previous)
    else:
        result=json.loads((V087/'result.json').read_text())
        save(PIPE/'v087_result_summary.json',{k:v for k,v in result.items() if k!='final'})
    run([PYTHON,ROOT/'adhoc/scripts/memory_guard.py','--log-dir',V088/'guard600','--seconds','7200',
         '--gpu-state',V088/'gpu_state.json','--',PYTHON,ROOT/'adhoc/scripts/train_v088_long.py','--run',V088,
         '--until','600','--seconds','6900'],V088/'train600.log','training_to_600')
    run([PYTHON,ROOT/'adhoc/scripts/run_v088_evaluation.py','--run',V088],V088/'evaluation.log','evaluating_600')
    result=json.loads((V088/'evaluation/comparison.json').read_text())
    status('completed',v087_exit_code=previous.get('exit_code'),v088_mean_saved=result['mean_saved'],
           v088_adopt=result['adopt'],elapsed_seconds=time.monotonic()-started)
    save(PIPE/'exit.json',{'exit_code':0 if previous.get('exit_code')==0 else 1,'completed_at':now()})


if __name__=='__main__':
    try:main()
    except BaseException as error:
        status('failed',error=repr(error));save(PIPE/'exit.json',{'exit_code':1,'error':repr(error),'completed_at':now()});raise
