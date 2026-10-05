#!/usr/bin/env python3
"""120周の重みを固定し、各条件を1回だけ測って初期方策を選ぶ。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

import numpy as np

from build_v090_board import build
from check_v089_board import compile_binary, trace
from run_v089_evaluation import validated_output, statistics
from v090_data import ROOT, RUN, EPOCHS, MODEL_SPECS, ACTIVE_SIZES, Dataset, save, sha, status, now


def load(path):
    return json.loads(path.read_text())


def wrapper(name, size, stochastic=False, diagnostic=False):
    path = ROOT / 'adhoc/bin' / f'{name}.cpp'
    text = f'// {name}.cpp\n'
    if stochastic: text += '#define V090_STOCHASTIC\n'
    if diagnostic: text += '#define V090_DIAGNOSTIC\n'
    text += f'#include "../../src/bin/v090_nn_{size}.cpp"\n'
    path.write_text(text)
    return path


def initial_states(root, size, role, condition, source, cases):
    directory = root / 'evaluation' / size / role / condition
    directory.mkdir(parents=True, exist_ok=True); marker = directory / 'result.json'
    if marker.exists():
        result = load(marker); assert result['solver_sha256'] == sha(source)
        return result
    name = f'v090_nn_{size}' if role == 'validation' and condition == 'greedy' else f'v090_{size}_{role}_{condition}'
    wrapper_path = None
    if name != source.stem: wrapper_path = wrapper(name, size, stochastic=condition == 'stochastic')
    inputs = directory / 'inputs'; inputs.mkdir(exist_ok=True)
    for case in cases:
        src = root / case['path']; assert sha(src) == case['sha256']
        shutil.copy2(src, inputs / f"{case['index']:06d}.txt")
    assert len(list(inputs.glob('*.txt'))) == len(cases) == 256
    save(directory / 'input_manifest.json', cases)
    started = directory / 'started.json'; output = ROOT / 'results/out' / name
    label = f'{root.name}_v090_{size}_{role}_{condition}'
    config = {'solver_sha256': sha(source), 'model_sha256': sha(root / 'models' / size / 'model.json'),
              'label': label, 'name': name, 'local': True, 'jobs': 20, 'input_sha256': sha(directory / 'input_manifest.json')}
    if not started.exists():
        assert not output.exists(), f'refuse to overwrite prior evaluation: {output}'
        for local in (False, True):
            compile_binary(name, directory / ('binary_local' if local else 'binary_judge'), local)
        save(started, config)
        status(root, 'evaluating_initial_states', size=size, role=role, condition=condition, cases=256, jobs=20)
        with (directory / 'eval.log').open('w') as log:
            proc = subprocess.run([sys.executable, ROOT / 'scripts/eval.py', name, inputs,
                                   '-j', '20', '--wait-lock', '--label', label], cwd=ROOT,
                                  stdout=log, stderr=subprocess.STDOUT)
        save(directory / 'exit.json', {'exit_code': proc.returncode, 'finished_at': now()})
        assert proc.returncode == 0, f'eval failed; inspect {directory}'
    else:
        # 評価後・集計前の中断は既存ログから復元する。途中の評価を丸ごと再実行しない。
        assert load(started) == config, 'evaluation identity changed'
    records = []
    with (ROOT / 'results/eval_records.jsonl').open() as stream:
        for line in stream:
            row = json.loads(line)
            if row['label'] == label: records.append(row)
    assert len(records) == len({r['case_name'] for r in records}) == 256, f'partial evaluation requires inspection: {directory}'
    assert len({r['run_id'] for r in records}) == 1
    save(directory / 'official_records.json', records)
    indexed = {r['case_name']: r for r in records}; rows = []
    archive = directory / 'outputs'
    if not archive.exists(): shutil.copytree(output, archive)
    for case in cases:
        filename = f"{case['index']:06d}.txt"; record = indexed[filename]
        assert record['status'] == 'ok' and record['local']
        result = validated_output(inputs / filename, (archive / filename).read_text())
        counts = trace((archive / (filename + '.err')).read_text())
        assert result['S'] == record['score'] and counts['E'] == result['E'] and counts['T'] == result['T']
        rows.append({'case': case['index'], 'elapsed_ms': record['elapsed'], 'trace': counts, **result})
    result = {'metrics': statistics(rows), 'rows': rows, 'all_legal': True,
              'solver_sha256': sha(source), 'completed_at': now()}
    save(marker, result)
    if wrapper_path: shutil.copy2(wrapper_path, directory / wrapper_path.name)
    return result


def suffixes(root, size, data, cases, source):
    directory = root / 'evaluation' / size / 'suffix'; directory.mkdir(parents=True, exist_ok=True)
    marker = directory / 'result.json'
    if marker.exists():
        result = load(marker); assert result['solver_sha256'] == sha(source)
        return result
    name = f'check_v090_{size}_stochastic'
    wrapper(name, size, stochastic=True, diagnostic=True)
    binary = directory / name; compile_binary(name, binary, True)
    def job(args):
        case, suffix = args; assert case['frames'] >= suffix
        fid = case['frame_start'] + case['frames'] - suffix
        stem = f"{case['index']:06d}_last{suffix}"
        done = directory / f'{stem}.json'
        if done.exists(): return load(done)
        state = np.array(data.states[fid]); snapshot = directory / f'{stem}.state'
        snapshot.write_text(' '.join(map(str, state)) + '\n')
        output, error = directory / f'{stem}.txt', directory / f'{stem}.err'
        running = directory / f'{stem}.started.json'
        assert not running.exists(), f'interrupted suffix requires inspection: {running}'
        save(running, {'started_at': now(), 'source_sha256': sha(source)})
        started = time.monotonic()
        proc = subprocess.run([binary, snapshot], input=(root / case['path']).read_text(),
                              capture_output=True, text=True, check=True, timeout=10.)
        elapsed_ms = (time.monotonic() - started) * 1000.
        output.write_text(proc.stdout); error.write_text(proc.stderr)
        result = validated_output(root / case['path'], proc.stdout, state); counts = trace(proc.stderr)
        assert counts['T'] == result['T'] and counts['E'] == result['E']
        row = dict(case=case['index'], frame=fid, teacher_suffix=suffix, elapsed_ms=elapsed_ms, trace=counts, **result)
        save(done, row)
        return row
    rows = []
    for suffix in (10, 50):
        status(root, 'evaluating_suffixes', size=size, suffix=suffix, jobs=20)
        with ThreadPoolExecutor(max_workers=20) as pool:
            rows.extend(pool.map(job, [(case, suffix) for case in cases]))
    result = {'metrics': {f'last{suffix}': statistics([r for r in rows if r['teacher_suffix'] == suffix]) for suffix in (10, 50)},
              'rows': rows, 'solver_sha256': sha(source), 'completed_at': now()}
    save(marker, result); return result


def evaluate_size(root, size):
    model_run = root / 'models' / size
    assert load(model_run / 'model.json')['epoch'] == EPOCHS
    assert load(model_run / 'numerical_check.json')['passed']
    source = ROOT / 'src/bin' / f'v090_nn_{size}.cpp'
    storage = build(model_run / 'model.json', source)
    directory = root / 'evaluation' / size; directory.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, directory / source.name); save(directory / 'storage.json', storage)
    data = Dataset(root / 'data'); cases = [c for c in data.cases if c['role'] == 'validation']
    all_rows = {condition: initial_states(root, size, 'validation', condition, source, cases) for condition in ('greedy', 'stochastic')}
    suffix = suffixes(root, size, data, cases, source)
    common = [(a, b) for a, b in zip(all_rows['greedy']['rows'], all_rows['stochastic']['rows']) if a['E'] == b['E'] == 0]
    result = {'initial': {k: v['metrics'] for k, v in all_rows.items()}, 'suffix': suffix['metrics'], 'all_legal': True,
              'common_completed': len(common),
              'mean_stochastic_minus_greedy_T_on_common': float(np.mean([b['T'] - a['T'] for a, b in common])) if common else None,
              'completed_at': now(), 'model_sha256': sha(model_run / 'model.json'), 'solver_sha256': sha(source)}
    save(directory / 'comparison.json', result); return result


def select(root):
    results = {size: load(root / 'evaluation' / size / 'comparison.json') for size in ACTIVE_SIZES}
    assert all(r['all_legal'] for r in results.values())
    ordered = sorted(results, key=lambda size: (-results[size]['initial']['greedy']['complete'],
                                              results[size]['initial']['greedy']['mean_S_all'],
                                              results[size]['initial']['greedy']['mean_elapsed_ms']))
    size = ordered[0]; validation = results[size]['initial']['greedy']
    decision = {'selected_size': size, 'order': ordered, 'validation': validation,
                'bc_homing_gate': validation['completion_rate'] >= .95,
                'submission_homing_gate': validation['completion_rate'] == 1.,
                'rule': 'more completed, lower mean S, shorter mean time; greedy validation only',
                'model_sha256': sha(root / 'models' / size / 'model.json')}
    selection = root / 'evaluation/selection.json'
    if selection.exists(): assert load(selection) == decision
    else: save(selection, decision)
    cases = [c for c in load(root / 'input_manifest.json') if c['role'] == 'test']
    test = initial_states(root, size, 'test', 'greedy', ROOT / 'src/bin' / f'v090_nn_{size}.cpp', cases)
    report = {'selection': decision, 'validation_by_size': results, 'test': test['metrics'], 'completed_at': now()}
    save(root / 'evaluation/comparison.json', report)
    status(root, 'bc_evaluation_completed', selected_size=size, test_complete=test['metrics']['complete'])
    return report


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--size', choices=list(MODEL_SPECS)); p.add_argument('--select', action='store_true')
    args = p.parse_args()
    assert bool(args.size) != args.select
    result = select(args.run.resolve()) if args.select else evaluate_size(args.run.resolve(), args.size)
    print(json.dumps(result, ensure_ascii=False), flush=True)
