#!/usr/bin/env python3
"""事前登録した実測報酬の学習、監査、補完とLNS込みの比較を順番に実行する。"""
import argparse
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v090_data import ROOT, save, sha, status, now
from v091_env import load
from v109_dp import configuration, INITIAL_SHA


def execute(root):
    pipe = root/'pipeline'; pipe.mkdir(parents=True, exist_ok=True)
    lock = (pipe/'lock').open('a'); fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code'] == 0: return
    assert sha(root/'initial/checkpoint.pt') == INITIAL_SHA
    names = ('v109_dp.py', 'train_v109_dp.py', 'assess_v109_dp.py', 'run_v109_pipeline.py', 'v109_completion.cpp.txt',
             'v102_learning.py', 'v101_compute.py', 'v099_complete.py', 'v104_extended.py', 'v092_stream.py', 'v091_env.py',
             'v090_data.py', 'v089_data.py', 'v096_selected.py', 'train_v091_ppo.py', 'train_v090_board.py',
             'train_v089_board.py', 'train_v077_rank.py', 'train_v080_scaling.py', 'build_v090_board.py',
             'check_v089_board.py', 'check_v098_complete.py', 'run_v089_evaluation.py', 'run_v105_hybrid.py',
             'v089_core.cpp.txt', 'v090_inference.cpp.txt', 'v090_search.cpp.txt', 'memory_guard.py')
    paths = [ROOT/'adhoc/scripts'/name for name in names]
    paths += [ROOT/'adhoc/bin'/name for name in ('v091_environment.cpp', 'v108_assisted_base.cpp', 'v109_dp_completion.cpp')]
    paths += [ROOT/'scripts/eval.py', ROOT/'scripts/build_solver.sh']
    identity = dict(config=configuration(), initial_checkpoint_sha256=INITIAL_SHA,
                    sources={str(p.relative_to(ROOT)): sha(p) for p in paths})
    if (pipe/'config.json').exists(): assert load(pipe/'config.json') == identity
    else:
        save(pipe/'config.json', identity)
        for path in paths+[ROOT/'notes/experiments/v109.md']:
            target = pipe/'frozen'/path.relative_to(ROOT); target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(path, target)
    def stage(name, script, args, marker):
        if marker.exists(): return
        for path, fingerprint in identity['sources'].items(): assert sha(ROOT/path) == fingerprint, path
        status(pipe, name, pid=os.getpid(), marker=str(marker))
        with (pipe/(name+'.log')).open('a') as stream:
            process = subprocess.run([sys.executable, ROOT/'adhoc/scripts'/script]+list(map(str, args)),
                                     cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
        save(pipe/(name+'_exit.json'), dict(exit_code=process.returncode, completed_at=now()))
        assert process.returncode == 0 and marker.exists(), f'{name} failed; inspect saved log'
    started = time.monotonic()
    try:
        stage('mechanism', 'train_v109_dp.py', ['--run', root, '--phase', 'mechanism', '--gpu-state', root/'gpu_state.json'], root/'mechanism/result.json')
        check = load(root/'mechanism/result.json')
        if not check['passed']:
            result = dict(mechanism=check, trained=False, accepted=False, reason='mechanism gate failed', completed_at=now())
        else:
            for phase, marker in (('before', 'audit/before/result.json'), ('train', 'training/result.json'), ('after', 'audit/after/result.json')):
                stage(phase, 'train_v109_dp.py', ['--run', root, '--phase', phase, '--gpu-state', root/'gpu_state.json'], root/marker)
            stage('assessment', 'assess_v109_dp.py', ['--run', root], root/'final/result.json')
            result = dict(mechanism=check, trained=True, training=load(root/'training/result.json'),
                          audit_before=load(root/'audit/before/result.json'), audit_after=load(root/'audit/after/result.json'),
                          assessment=load(root/'assessment.json'), final=load(root/'final/result.json'), completed_at=now())
        save(root/'result.json', result)
        save(pipe/'exit.json', dict(exit_code=0, seconds=time.monotonic()-started, finished_at=now()))
        status(pipe, 'completed', next_stage='verify records, write experiment/backlog, commit/push, stop heartbeat')
    except BaseException as error:
        save(pipe/'exit.json', dict(exit_code=1, error=repr(error), traceback=traceback.format_exc(), finished_at=now()))
        status(pipe, 'failed', error=repr(error)); raise


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, required=True); a = p.parse_args(); execute(a.run.resolve())
