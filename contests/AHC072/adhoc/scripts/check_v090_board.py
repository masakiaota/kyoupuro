#!/usr/bin/env python3
"""同じ量子化重みのPython/C++、D4変換、再生と計時を照合する。"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import time

import numpy as np
import torch

from build_v090_board import build, quantize
from check_v089_board import compile_binary, trace, transformed_input
from train_v090_board import model_new, tensors
from v090_data import ROOT, RUN, MODEL_SPECS, Dataset, Geometry, remaining, save, sha, now


def check(root, size, mechanism=False):
    run = root / 'models' / size
    label = 'preflight' if mechanism else 'numerical'
    directory = run / label; directory.mkdir(parents=True, exist_ok=True)
    model_path = run / ('mechanism_model.json' if mechanism else 'model.json')
    marker = run / f'{label}_check.json'
    if marker.exists():
        result = json.loads(marker.read_text())
        assert result['passed'] and result['model_sha256'] == sha(model_path)
        return result
    name = f'check_v090_{size}_{label}'
    source = ROOT / 'adhoc/bin' / f'{name}.cpp'
    storage = build(model_path, source, diagnostic=True)
    local, judge = directory / 'checker_local', directory / 'checker_judge'
    compile_binary(name, local, True); compile_binary(name, judge, False)
    shutil.copy2(source, directory / source.name); source.unlink()
    data = Dataset(root / 'data'); model = model_new(size, 'cpu'); original = model_new(size, 'cpu')
    parameters = json.loads(model_path.read_text())['parameters']
    restored, _, _ = quantize(parameters)
    model.load_state_dict({k: torch.from_numpy(v) for k, v in restored.items()}); model.eval()
    original.load_state_dict({k: torch.tensor(v) for k, v in parameters.items()}); original.eval()
    cases = [c for c in data.cases if c['role'] == 'train'][:4]
    ids = [c['frame_start'] + c['frames'] // 2 for c in cases]
    errors = dict(board_features=0., action_features=0., logits=0., value=0.)
    quantization = dict(max_logits=0., max_value=0., top1_changed=0, states=0)
    timings = []; records = []
    for fid in ids:
        case = data.by_index[int(data.case_ids[fid])]; geo = Geometry(root / case['path'])
        for group in range(8):
            mapping = data.maps[geo.N][group]
            inp = directory / 'transformed_input.txt'; inp.write_text(transformed_input(geo, mapping))
            state = np.zeros(400, np.uint32); state[mapping] = data.states[fid]
            snapshot = directory / 'snapshot.txt'; snapshot.write_text(' '.join(map(str, state)) + '\n')
            raw = data.batch([fid], groups=[group]); batch = tensors(raw, 'cpu')
            with torch.no_grad():
                py_logits, py_value = model(batch); float_logits, float_value = original(batch)
            quantization['max_logits'] = max(quantization['max_logits'], (py_logits - float_logits).abs().max().item())
            quantization['max_value'] = max(quantization['max_value'], (py_value - float_value).abs().max().item())
            quantization['top1_changed'] += int(py_logits.argmax(-1).item() != float_logits.argmax(-1).item())
            quantization['states'] += 1
            codes = data.codes[int(data.offsets[fid]):int(data.offsets[fid + 1])].astype(np.int64)
            dirs = data.dirs[geo.N][group][(codes >> 12) & 3]
            changed = mapping[codes & 511] | (codes & (7 << 9)) | (dirs << 12) | (codes & (7 << 14))
            for binary in ([local, judge] if group == 0 else [local]):
                proc = subprocess.run([binary, snapshot, 'dump'], input=inp.read_text(), text=True,
                                      capture_output=True, check=True, timeout=10.)
                cpp = json.loads(proc.stdout); positions = {code: i for i, code in enumerate(cpp['codes'])}
                assert len(positions) == len(changed) and set(positions) == set(changed.tolist())
                order = np.array([positions[int(code)] for code in changed])
                current = {'board_features': np.max(np.abs(np.array(cpp['x']).reshape(400, 40) - raw['x'][0].reshape(40, 400).T)),
                           'action_features': np.max(np.abs(np.array(cpp['features']).reshape(-1, 16)[order] - raw['features'][0])),
                           'logits': np.max(np.abs(np.array(cpp['logits'])[order] - py_logits[0].numpy())),
                           'value': abs(cpp['value'] - float(py_value[0]))}
                for key, value in current.items(): errors[key] = max(errors[key], float(value))
                timings.append({'frame': fid, 'group': group, 'local': binary == local,
                                'seconds': cpp['inference_seconds']})
            original_after = np.array(data.states[fid], copy=True)
            teacher = int(codes[int(data.targets[fid])]); p = teacher & 511
            geo.apply(original_after, f'{p // 20} {p % 20} {(teacher >> 9) & 7} {"UDLR"[(teacher >> 12) & 3]} {((teacher >> 14) & 7) + 1}')
            target = int(changed[int(data.targets[fid])]); p = target & 511
            Geometry(inp).apply(state, f'{p // 20} {p % 20} {(target >> 9) & 7} {"UDLR"[(target >> 12) & 3]} {((target >> 14) & 7) + 1}')
            expected = np.zeros(400, np.uint32); expected[mapping] = original_after
            assert np.array_equal(state, expected), (fid, group, 'D4 transition')
        records.append({'frame': fid, 'groups': 8, 'nonlocal_group0': True})
    passed = errors['board_features'] < 2e-6 and errors['action_features'] < 2e-6 and errors['logits'] < .003 and errors['value'] < .03
    assert passed, errors
    searches = []
    if mechanism:
        for probe_index, fid in enumerate(ids[:2]):
            case = data.by_index[int(data.case_ids[fid])]; geo = Geometry(root / case['path'])
            snapshot = directory / f'search_{fid}.txt'; snapshot.write_text(' '.join(map(str, data.states[fid])) + '\n')
            args = [local, snapshot] + (['deadline'] if probe_index == 1 else [])
            started = time.monotonic()
            proc = subprocess.run(args, input=(root / case['path']).read_text(), text=True,
                                  capture_output=True, check=True, timeout=5.)
            elapsed = time.monotonic() - started; counts = trace(proc.stderr)
            state = np.array(data.states[fid], copy=True)
            for line in proc.stdout.splitlines(): geo.apply(state, line)
            assert counts['E'] == remaining(state) and counts['T'] == len(proc.stdout.splitlines())
            assert counts['inferences'] > 0 and elapsed < 2.
            assert probe_index == 0 or counts['deadline'] == 1
            searches.append(dict(frame=fid, elapsed_seconds=elapsed, timer_fraction=.01 if probe_index else 1., trace=counts))
    result = dict(passed=passed, max_error=errors, checks=records, searches=searches, timings=timings,
                  quantization_difference=quantization, storage=storage,
                  model_sha256=sha(model_path), completed_at=now())
    save(marker, result)
    print(json.dumps({k: v for k, v in result.items() if k not in ('checks', 'searches', 'timings')}), flush=True)
    return result


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--size', choices=list(MODEL_SPECS), required=True); p.add_argument('--mechanism', action='store_true')
    args = p.parse_args(); torch.set_num_threads(2); torch.set_num_interop_threads(2)
    check(args.run.resolve(), args.size, args.mechanism)
