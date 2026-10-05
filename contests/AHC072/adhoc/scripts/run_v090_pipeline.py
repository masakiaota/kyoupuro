#!/usr/bin/env python3
"""固定した小型NNの学習、数値照合、未学習入力の評価を順に完了させる。"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback

from v090_data import ROOT, RUN, ACTIVE_SIZES, BATCH, EPOCHS, MODEL_SPECS, save, sha, status, now


def load(path):
    return json.loads(path.read_text())


def execute(root, seconds):
    pipe = root / 'pipeline'; pipe.mkdir(parents=True, exist_ok=True)
    # 手動確認とheartbeatが重なっても、同じ学習を二重に起動しない。
    lock = (pipe / 'lock').open('a')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    config_path = pipe / 'config.json'
    if not config_path.exists():
        scripts = ['v090_data.py', 'train_v090_board.py', 'build_v090_board.py', 'check_v090_board.py',
                   'run_v090_evaluation.py', 'run_v090_pipeline.py', 'v090_inference.cpp.txt', 'v090_search.cpp.txt',
                   'v089_core.cpp.txt', 'v089_data.py', 'train_v089_board.py', 'check_v089_board.py',
                   'build_v089_board.py', 'run_v089_evaluation.py', 'train_v077_rank.py', 'train_v080_scaling.py',
                   'memory_guard.py']
        paths = [ROOT / 'adhoc/scripts' / name for name in scripts]
        paths += [ROOT / 'scripts/build_solver.sh', ROOT / 'scripts/eval.py', ROOT / 'notes/experiments/v090.md']
        fingerprints = {}
        for source in paths:
            relative = source.relative_to(ROOT); dest = pipe / 'frozen' / relative
            dest.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(source, dest)
            # ノートは作業記録として更新するので、実行コードだけを実行前の固定対象にする。
            if relative.parts[0] != 'notes': fingerprints[str(relative)] = sha(source)
        config = {'active_sizes': list(ACTIVE_SIZES), 'specs': {s: MODEL_SPECS[s] for s in ACTIVE_SIZES},
                  'epochs': EPOCHS, 'batch': BATCH, 'source_sha256': fingerprints,
                  'dataset_sha256': sha(root / 'data/dataset.json'), 'created_at': now(),
                  'work_deadline_jst': '2026-10-05T01:00:00+09:00', 'training_prefetch_batches': 1}
        save(config_path, config)
    config = load(config_path)
    assert config['active_sizes'] == list(ACTIVE_SIZES) and config['epochs'] == EPOCHS and config['batch'] == BATCH
    assert config['dataset_sha256'] == sha(root / 'data/dataset.json')
    def verify_sources():
        for relative, digest in config['source_sha256'].items(): assert sha(ROOT / relative) == digest, f'source changed: {relative}'
    verify_sources()
    if not (pipe / 'dataset_verified.json').exists():
        status(pipe, 'checking_saved_arrays', pid=os.getpid())
        for name, digest in load(root / 'data/dataset.json')['array_sha256'].items():
            assert sha(root / 'data' / f'{name}.npy') == digest, name
        save(pipe / 'dataset_verified.json', {'dataset_sha256': config['dataset_sha256'], 'verified_at': now()})
    else:
        assert load(pipe / 'dataset_verified.json')['dataset_sha256'] == config['dataset_sha256']
    started = time.monotonic()
    def stage(label, script, args, marker):
        verify_sources()
        if marker.exists():
            save(pipe / f'{label}_reused.json', {'marker': str(marker), 'sha256': sha(marker), 'checked_at': now()})
            return
        remaining = int(seconds - (time.monotonic() - started))
        assert remaining > 0, 'pipeline time limit'
        command = [sys.executable, ROOT / 'adhoc/scripts' / script, '--run', root] + args
        if script == 'train_v090_board.py': command += ['--seconds', str(remaining)]
        status(pipe, label, pid=os.getpid(), marker=str(marker), remaining_seconds=remaining)
        status(root, label, pipeline_pid=os.getpid())
        with (pipe / f'{label}.log').open('a') as log:
            proc = subprocess.run(list(map(str, command)), cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        save(pipe / f'{label}_exit.json', {'exit_code': proc.returncode, 'finished_at': now()})
        assert proc.returncode == 0 and marker.exists(), f'{label} failed; inspect {pipe / (label + ".log")}'
    for size in ACTIVE_SIZES:
        model = root / 'models' / size
        assert load(model / 'learning_check.json')['passed'] and load(model / 'preflight_check.json')['passed']
        stage(f'train_{size}', 'train_v090_board.py', ['--size', size], model / 'result.json')
        stage(f'numerical_{size}', 'check_v090_board.py', ['--size', size], model / 'numerical_check.json')
        stage(f'evaluate_{size}', 'run_v090_evaluation.py', ['--size', size], root / 'evaluation' / size / 'comparison.json')
    stage('held_out_baseline', 'run_v090_evaluation.py', ['--select'], root / 'evaluation/comparison.json')
    status(pipe, 'completed', pid=os.getpid(), next_stage='record v090 results and preregister authorized v091 reinforcement learning')
    save(pipe / 'exit.json', {'exit_code': 0, 'finished_at': now(), 'elapsed_seconds': time.monotonic() - started})


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--seconds', type=int, default=57500); args = p.parse_args(); args.run = args.run.resolve()
    try:
        execute(args.run, args.seconds)
    except BlockingIOError:
        print('an existing pipeline owns the lock; no work started', file=sys.stderr); sys.exit(76)
    except BaseException as error:
        directory = args.run / 'pipeline'; directory.mkdir(parents=True, exist_ok=True)
        save(directory / 'exit.json', {'exit_code': 1, 'finished_at': now(), 'error': repr(error)})
        traceback.print_exc(); sys.exit(1)
