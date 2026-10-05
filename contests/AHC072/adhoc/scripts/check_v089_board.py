#!/usr/bin/env python3
"""実盤面・D4変換・C++推論・探索機構を固定した盤面で照合する。"""
import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import time

import numpy as np
import torch

from build_v089_board import build
from train_v089_board import model_new, tensors
from v089_data import ROOT, SOURCE, RUN, Dataset, Geometry, remaining, save, sha, now


def compile_binary(name, destination, local=True):
    command = [ROOT / 'scripts/build_solver.sh'] + ([] if local else ['--no-local']) + [name]
    subprocess.run(command, cwd=ROOT, check=True, stdout=subprocess.PIPE)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / 'target/release' / name, destination)


def trace(text):
    return {key: int(value) for key, value in re.findall(r'\[summary.count\] (\w+)=(-?\d+)', text)}


def transformed_input(geo, mapping):
    grid = np.full(400, '#', dtype='<U1')
    for i, row in enumerate(geo.C):
        for j, ch in enumerate(row): grid[mapping[i * 20 + j]] = ch
    return f'{geo.N} {geo.K}\n' + '\n'.join(''.join(grid[i * 20:i * 20 + geo.N]) for i in range(geo.N)) + '\n'


def check(run, mechanism=False):
    label = 'preflight' if mechanism else 'numerical'
    directory = run / label; directory.mkdir(parents=True, exist_ok=True)
    model_path = run / ('mechanism_model.json' if mechanism else 'model.json')
    if mechanism:
        source = ROOT / 'adhoc/bin/v089_mechanism.cpp'
        build(model_path, source)
        source.write_text('// v089_mechanism.cpp\n#define V089_DIAGNOSTIC\n' + source.read_text())
        name = 'v089_mechanism'
    else:
        source = ROOT / 'src/bin/v089_nn_board.cpp'; build(model_path, source); name = 'check_v089_board'
    local, nonlocal_ = directory / 'checker_local', directory / 'checker_judge'
    compile_binary(name, local, True); compile_binary(name, nonlocal_, False)
    if mechanism:
        shutil.copy2(source, directory / source.name); source.unlink()
    data = Dataset(run); model = model_new('cpu')
    parameters = json.loads(model_path.read_text())['parameters']
    model.load_state_dict({k: torch.tensor(v) for k, v in parameters.items()}); model.eval()
    cases = [data.cases[i] for i in (1, 2, 0, 4)]
    ids = [c['frame_start'] + min(c['frames'] // 2, c['frames'] - 10) for c in cases]
    errors = dict(board_features=0., action_features=0., logits=0., value=0.)
    records = []
    for fid in ids:
        case = data.cases[int(data.case_ids[fid])]; geo = Geometry(SOURCE / case['path'])
        for group in range(8):
            mapping = data.maps[geo.N][group]
            inp = directory / 'transformed_input.txt'; inp.write_text(transformed_input(geo, mapping))
            state = np.zeros(400, np.uint32); state[mapping] = data.states[fid]
            snapshot = directory / 'snapshot.txt'; snapshot.write_text(' '.join(map(str, state)) + '\n')
            raw = data.batch([fid], groups=[group]); batch = tensors(raw, 'cpu')
            with torch.no_grad(): py_logits, py_value = model(batch)
            codes = data.codes[int(data.offsets[fid]):int(data.offsets[fid + 1])].astype(np.int64)
            dirs = data.dirs[geo.N][group][(codes >> 12) & 3]
            changed = mapping[codes & 511] | (codes & (7 << 9)) | (dirs << 12) | (codes & (7 << 14))
            for binary in ([local, nonlocal_] if group == 0 else [local]):
                proc = subprocess.run([binary, snapshot, 'dump'], input=inp.read_text(), text=True,
                                      capture_output=True, check=True)
                cpp = json.loads(proc.stdout)
                positions = {code: i for i, code in enumerate(cpp['codes'])}
                assert len(positions) == len(changed) and set(positions) == set(changed.tolist())
                order = np.array([positions[int(code)] for code in changed])
                x = np.array(cpp['x']).reshape(400, 40)
                features = np.array(cpp['features']).reshape(-1, 16)[order]
                current = {'board_features': np.max(np.abs(x - raw['x'][0].reshape(40, 400).T)),
                           'action_features': np.max(np.abs(features - raw['features'][0])),
                           'logits': np.max(np.abs(np.array(cpp['logits'])[order] - py_logits[0].numpy())),
                           'value': abs(cpp['value'] - float(py_value[0]))}
                for key, value in current.items(): errors[key] = max(errors[key], float(value))
            # 回転・反転した教師の適用結果も独立したPython再生で確認する。
            original_after = np.array(data.states[fid], copy=True)
            teacher = int(codes[int(data.targets[fid])]); p = teacher & 511
            line = f'{p // 20} {p % 20} {(teacher >> 9) & 7} {"UDLR"[(teacher >> 12) & 3]} {((teacher >> 14) & 7) + 1}'
            geo.apply(original_after, line)
            target = int(changed[int(data.targets[fid])]); p = target & 511
            changed_line = f'{p // 20} {p % 20} {(target >> 9) & 7} {"UDLR"[(target >> 12) & 3]} {((target >> 14) & 7) + 1}'
            rotated = Geometry(inp); rotated.apply(state, changed_line)
            expected = np.zeros(400, np.uint32); expected[mapping] = original_after
            assert np.array_equal(state, expected), (fid, group, 'D4 transition')
        records.append({'frame': fid, 'groups': 8, 'nonlocal_group0': True})
    passed = errors['board_features'] < 2e-6 and errors['action_features'] < 2e-6 and errors['logits'] < .002 and errors['value'] < .02
    assert passed, errors
    searches = []
    if mechanism:
        for probe_index, fid in enumerate(ids[:2]):
            case = data.cases[int(data.case_ids[fid])]; geo = Geometry(SOURCE / case['path'])
            snapshot = directory / f'search_{fid}.txt'; snapshot.write_text(' '.join(map(str, data.states[fid])) + '\n')
            started = time.monotonic()
            args = [local, snapshot] + (['deadline'] if probe_index == 1 else [])
            proc = subprocess.run(args, input=(SOURCE / case['path']).read_text(), text=True,
                                  capture_output=True, check=True, timeout=3.)
            elapsed = time.monotonic() - started; observed = trace(proc.stderr)
            state = np.array(data.states[fid], copy=True)
            for line in proc.stdout.splitlines(): geo.apply(state, line)
            assert observed['E'] == remaining(state) and observed['T'] == len(proc.stdout.splitlines())
            assert observed['max_width'] == 4 and observed['duplicates'] > 0 and observed['inferences'] > 1
            assert (probe_index == 0 or observed['deadline'] == 1) and elapsed < 2.
            searches.append(dict(frame=fid, elapsed_seconds=elapsed, timer_fraction=.01 if probe_index == 1 else 1., trace=observed))
    result = dict(passed=passed, max_error=errors, checks=records, searches=searches,
                  model_sha256=sha(model_path), completed_at=now())
    save(run / f'{label}_check.json', result); print(json.dumps(result), flush=True)


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN); p.add_argument('--mechanism', action='store_true')
    args = p.parse_args(); torch.set_num_threads(2); torch.set_num_interop_threads(2)
    check(args.run.resolve(), args.mechanism)
