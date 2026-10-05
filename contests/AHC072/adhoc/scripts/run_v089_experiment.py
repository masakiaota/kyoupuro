#!/usr/bin/env python3
"""学習、重みのC++照合、固定評価を順次実行する。失敗時は記録して停止する。"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from v089_data import ROOT, RUN, save, sha, status, now

SOURCES = ['adhoc/scripts/v089_core.cpp.txt', 'adhoc/scripts/v089_search.cpp.txt',
           'adhoc/scripts/v089_data.py', 'adhoc/scripts/train_v089_board.py',
           'adhoc/scripts/build_v089_board.py', 'adhoc/scripts/check_v089_board.py',
           'adhoc/scripts/run_v089_evaluation.py', 'adhoc/scripts/run_v089_experiment.py',
           'adhoc/bin/check_v089_board.cpp', 'adhoc/bin/check_v089_greedy.cpp', 'adhoc/bin/v089_nn_greedy.cpp',
           'adhoc/scripts/memory_guard.py', 'adhoc/scripts/train_v077_rank.py', 'adhoc/scripts/train_v080_scaling.py',
           'scripts/eval.py', 'scripts/build_solver.sh']


def main(run):
    pipe = run / 'pipeline'; pipe.mkdir(exist_ok=False)
    frozen = {path: sha(ROOT / path) for path in SOURCES}; save(pipe / 'source_sha256.json', frozen)
    started = time.monotonic()
    def verify():
        for path, expected in frozen.items():
            if sha(ROOT / path) != expected: raise RuntimeError(f'source changed during experiment: {path}')
    def execute(command, stage, log):
        verify()
        with (run / log).open('w') as out:
            child = subprocess.Popen(list(map(str, command)), cwd=ROOT, stdin=subprocess.DEVNULL,
                                     stdout=out, stderr=subprocess.STDOUT)
            save(pipe / 'status.json', {'stage': stage, 'pid': child.pid, 'updated_at': now()})
            status(run, stage, pid=child.pid)
            code = child.wait()
        if code: raise RuntimeError(f'{stage} exited {code}; see {log}')
    try:
        assert json.loads((run / 'learning_check.json').read_text())['passed']
        assert json.loads((run / 'preflight_check.json').read_text())['passed']
        for name, expected in json.loads((run / 'dataset.json').read_text())['array_sha256'].items():
            assert sha(run / f'{name}.npy') == expected, name
        gpu = run / 'gpu_state.json'; save(gpu, {'active': False, 'driver_bytes': 0})
        execute([sys.executable, ROOT / 'adhoc/scripts/memory_guard.py', '--log-dir', run / 'guard_train',
                 '--seconds', '14400', '--gpu-state', gpu, '--', sys.executable,
                 ROOT / 'adhoc/scripts/train_v089_board.py', '--run', run, '--seconds', '14000'],
                'training_launch', 'train.log')
        execute([sys.executable, ROOT / 'adhoc/scripts/check_v089_board.py', '--run', run],
                'checking_final_cpp', 'numerical.log')
        execute([sys.executable, ROOT / 'adhoc/scripts/run_v089_evaluation.py', '--run', run],
                'evaluating', 'evaluation.log')
        save(pipe / 'exit.json', {'exit_code': 0, 'completed_at': now(), 'seconds': time.monotonic() - started})
        save(pipe / 'status.json', {'stage': 'completed', 'updated_at': now()})
    except BaseException as error:
        save(pipe / 'exit.json', {'exit_code': 1, 'completed_at': now(), 'error': repr(error)})
        save(pipe / 'status.json', {'stage': 'failed', 'updated_at': now(), 'error': repr(error)})
        status(run, 'failed', error=repr(error))
        raise


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN); args = p.parse_args()
    main(args.run.resolve())
