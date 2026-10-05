#!/usr/bin/env python3
"""固定予算のPPO、C++照合、未学習入力の比較を再開可能に実行する。"""
import argparse
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback

from v091_env import ROOT, BC_RUN, RUN, CONFIG, load, save, sha, now
from v090_data import status


def execute(root):
    pipe = root / 'pipeline'; pipe.mkdir(parents=True, exist_ok=True)
    lock = (pipe / 'lock').open('a'); fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    assert load(root / 'mechanism/result.json')['passed']
    assert load(root / 'mechanism_training/result.json')['iterations'] == 2
    config_path = pipe / 'config.json'
    paths = [ROOT / 'adhoc/scripts' / name for name in (
        'v091_env.py', 'train_v091_ppo.py', 'check_v091_ppo.py', 'run_v091_evaluation.py', 'run_v091_pipeline.py',
        'v090_data.py', 'train_v090_board.py', 'build_v090_board.py', 'v090_inference.cpp.txt', 'v090_search.cpp.txt',
        'v089_core.cpp.txt', 'v089_data.py', 'train_v089_board.py', 'check_v089_board.py', 'build_v089_board.py',
        'run_v089_evaluation.py', 'train_v077_rank.py', 'train_v080_scaling.py', 'memory_guard.py')]
    paths += [ROOT / p for p in ('adhoc/bin/v091_environment.cpp', 'scripts/build_solver.sh', 'scripts/eval.py')]
    identity = dict(config=CONFIG, source_sha256={str(p.relative_to(ROOT)): sha(p) for p in paths},
                    initial_model_sha256=sha(BC_RUN / 'models/small/model.json'), dataset_sha256=sha(BC_RUN / 'data/dataset.json'))
    if config_path.exists(): assert load(config_path) == identity
    else:
        save(config_path, identity)
        for source in paths + [ROOT / 'notes/experiments/v091.md']:
            target = pipe / 'frozen' / source.relative_to(ROOT); target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(source, target)
    started = time.monotonic()
    for label, script, args, marker in (
        ('train', 'train_v091_ppo.py', [], root / 'training/result.json'),
        ('numerical', 'check_v091_ppo.py', ['--numerical'], root / 'numerical/result.json'),
        ('evaluate', 'run_v091_evaluation.py', [], root / 'evaluation/comparison.json')):
        for relative, digest in identity['source_sha256'].items(): assert sha(ROOT / relative) == digest, f'code changed: {relative}'
        if marker.exists(): continue
        status(pipe, label, pid=os.getpid(), marker=str(marker)); status(root, label, pipeline_pid=os.getpid())
        with (pipe / f'{label}.log').open('a') as stream:
            process = subprocess.run([sys.executable, ROOT / 'adhoc/scripts' / script, '--run', root] + args,
                                     cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
        save(pipe / f'{label}_exit.json', dict(exit_code=process.returncode, finished_at=now()))
        assert process.returncode == 0 and marker.exists(), f'{label} failed; inspect {pipe}'
    status(pipe, 'completed', pid=os.getpid(), next_stage='record v091 results and report the preregistered decision')
    save(pipe / 'exit.json', dict(exit_code=0, finished_at=now(), seconds=time.monotonic()-started))


if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);args=p.parse_args();root=args.run.resolve()
    try: execute(root)
    except BlockingIOError:
        print('existing pipeline holds the lock',file=sys.stderr);sys.exit(76)
    except BaseException as error:
        (root / 'pipeline').mkdir(parents=True,exist_ok=True)
        save(root / 'pipeline/exit.json',dict(exit_code=1,error=repr(error),finished_at=now()))
        traceback.print_exc();sys.exit(1)
