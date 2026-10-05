#!/usr/bin/env python3
"""全教師を実盤面へ変換する。入力単位で再開し、保留入力を学習へ入れない。"""
import argparse
from concurrent.futures import ProcessPoolExecutor, as_completed
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

import numpy as np

from v089_data import ROOT, Geometry, DIRECTIONS, d4_maps, remaining, now, save, sha, status

RUN = ROOT / 'results/nn_rank/v090/20261003_scaling_studio'
SEED, BATCH, EPOCHS = 90002, 128, 120
ACTIVE_SIZES = ('small',)  # 提出期限とC++推論費用から、本学習は小型1種類に固定。
MODEL_SPECS = {'small': {'width': 48, 'depth': 4, 'hidden': 64},
               'medium': {'width': 96, 'depth': 6, 'hidden': 96},
               'large': {'width': 160, 'depth': 8, 'hidden': 160}}
ARRAY_NAMES = ('states', 'offsets', 'targets', 'codes', 'features', 'case_ids', 'remaining', 'static', 'layer')


def load(path):
    return json.loads(path.read_text())


def case_prepare(task):
    run, item = task
    directory = run / 'data/raw' / f"{item['index']:06d}"
    inp = run / item['path']; solution = run / 'cases' / directory.name / 'best.txt'
    assert item['role'] in ('train', 'validation')
    assert sha(inp) == item['sha256']
    record = load(solution.parent / 'complete.json')
    assert sha(solution) == record['best_sha256']
    marker = directory / 'complete.json'
    if marker.exists():
        previous = load(marker)
        assert previous['input_sha256'] == item['sha256'] and previous['solution_sha256'] == sha(solution)
        for name, digest in previous['raw_sha256'].items():
            assert sha(directory / name) == digest, (directory, name)
        return previous
    if directory.exists():
        # 変換途中のファイルだけを保存してやり直す。元の教師と完成済み配列は変更しない。
        dest = run / 'data/interrupted' / f"{directory.name}_{time.time_ns()}"
        dest.parent.mkdir(parents=True, exist_ok=True); directory.rename(dest)
    if shutil.disk_usage(run).free < 20 * 1024**3:
        raise RuntimeError('less than 20 GiB disk space before data conversion')
    proc = subprocess.run([run / 'data/prepare_boards', inp, solution, directory],
                          text=True, capture_output=True, check=True)
    sizes = json.loads(proc.stdout); T, A = sizes['frames'], sizes['actions']
    assert T == record['T'] and sizes['E'] == 0
    states = np.memmap(directory / 'states.raw', mode='r', dtype='<u4', shape=(T, 400))
    offsets = np.fromfile(directory / 'offsets.raw', dtype='<u8')
    targets = np.fromfile(directory / 'targets.raw', dtype='<i4')
    codes = np.memmap(directory / 'codes.raw', mode='r', dtype='<u4', shape=(A,))
    assert len(offsets) == T + 1 and offsets[-1] == A and len(targets) == T
    assert np.all(np.diff(offsets) > 0)
    geo = Geometry(inp); state = geo.initial.copy()
    lines = [s for s in solution.read_text().splitlines() if s.strip()]
    for t, line in enumerate(lines):
        assert np.array_equal(state, states[t]), (item['index'], t, 'state mismatch')
        assert 0 <= targets[t] < offsets[t + 1] - offsets[t]
        code = geo.apply(state, line)
        assert codes[int(offsets[t]) + int(targets[t])] == code
    assert remaining(state) == 0 and len(lines) == T
    static = np.zeros((400, 8), np.float32); layer = np.zeros((13, 400, 3), np.float32)
    rows, cols = np.divmod(np.arange(400), 20)
    static[:, 0] = geo.floor; static[:, 1] = geo.nest >= 0
    static[:, 2] = rows / 20.; static[:, 3] = cols / 20.
    static[:, 6] = geo.N / 20.; static[:, 7] = geo.K / 12.; static[~geo.floor] = 0
    for c in range(1, geo.K + 1):
        layer[c, :, 0] = (geo.goal[c, 0] - rows) / 20.
        layer[c, :, 1] = (geo.goal[c, 1] - cols) / 20.
        layer[c, :, 2] = geo.distance[c] / 40.
    static.tofile(directory / 'static.raw'); layer.tofile(directory / 'layer.raw')
    result = dict(item, frames=T, actions=A, input_sha256=item['sha256'],
                  solution_sha256=sha(solution), baseline_T=record['baseline_T'],
                  raw_sha256={p.name: sha(p) for p in sorted(directory.glob('*.raw'))})
    save(marker, result)
    return result


def prepare(run, limit=0):
    assert load(run / 'teacher_exit.json')['exit_code'] == 0
    manifest = load(run / 'input_manifest.json')
    items = [c for c in manifest if c['role'] != 'test']
    assert len(items) == 4352 and sum(c['role'] == 'train' for c in items) == 4096
    directory = run / 'data'; directory.mkdir(exist_ok=True)
    if (directory / 'dataset.json').exists():
        print('verified dataset already exists; no conversion started', flush=True)
        return
    binary = ROOT / 'target/release/prepare_v090_boards'
    if not (directory / 'prepare_boards').exists():
        shutil.copy2(binary, directory / 'prepare_boards')
    assert sha(binary) == sha(directory / 'prepare_boards')
    if limit:
        items = items[:limit]
    started = time.monotonic(); cases = []
    status(run, 'preparing_dataset', completed=0, total=len(items), workers=20)
    with ProcessPoolExecutor(max_workers=20) as pool:
        jobs = [pool.submit(case_prepare, (run, item)) for item in items]
        for future in as_completed(jobs):
            cases.append(future.result())
            if len(cases) % 32 == 0 or len(cases) == len(items):
                elapsed = time.monotonic() - started
                status(run, 'preparing_dataset', completed=len(cases), total=len(items), workers=20,
                       elapsed_seconds=elapsed, remaining_seconds=elapsed * (len(items) - len(cases)) / len(cases))
    if limit:
        save(directory / 'pilot.json', {'inputs': len(cases), 'seconds': time.monotonic() - started,
                                       'frames': sum(c['frames'] for c in cases), 'actions': sum(c['actions'] for c in cases)})
        return
    cases.sort(key=lambda c: c['index'])
    total = sum(c['frames'] for c in cases); actions = sum(c['actions'] for c in cases)
    case_slots = len(manifest)
    specs = [('states', 'uint32', (total, 400)), ('offsets', 'int64', (total + 1,)),
             ('targets', 'int64', (total,)), ('codes', 'uint32', (actions,)),
             ('features', 'float32', (actions, 16)), ('case_ids', 'int16', (total,)),
             ('remaining', 'float32', (total,)), ('static', 'float32', (case_slots, 400, 8)),
             ('layer', 'float32', (case_slots, 13, 400, 3))]
    required = sum(np.prod(shape, dtype=np.int64) * np.dtype(dtype).itemsize for _, dtype, shape in specs)
    if shutil.disk_usage(run).free < required + 20 * 1024**3:
        raise RuntimeError(f'insufficient disk for consolidation: {required} bytes plus 20 GiB required')
    status(run, 'consolidating_dataset', frames=total, actions=actions, array_bytes=int(required))
    arrays = {name: np.lib.format.open_memmap(directory / f'{name}.partial.npy', mode='w+', dtype=dtype, shape=shape)
              for name, dtype, shape in specs}
    arrays['static'][:] = 0; arrays['layer'][:] = 0
    frame_base = action_base = 0
    for item in cases:
        T, A = item['frames'], item['actions']; raw = directory / 'raw' / f"{item['index']:06d}"
        item['frame_start'] = frame_base
        for name, dtype, shape in [('states', '<u4', (T, 400)), ('targets', '<i4', (T,)),
                                   ('codes', '<u4', (A,)), ('features', '<f4', (A, 16))]:
            values = np.memmap(raw / f'{name}.raw', mode='r', dtype=dtype, shape=shape)
            base = action_base if name in ('codes', 'features') else frame_base
            arrays[name][base:base + shape[0]] = values
            del values
        arrays['offsets'][frame_base:frame_base + T + 1] = np.fromfile(raw / 'offsets.raw', dtype='<u8') + action_base
        arrays['case_ids'][frame_base:frame_base + T] = item['index']
        arrays['remaining'][frame_base:frame_base + T] = np.arange(T, 0, -1)
        for name, shape in [('static', (400, 8)), ('layer', (13, 400, 3))]:
            arrays[name][item['index']] = np.fromfile(raw / f'{name}.raw', dtype='<f4').reshape(shape)
        frame_base += T; action_base += A
    for a in arrays.values(): a.flush()
    arrays.clear()
    for name in ARRAY_NAMES: (directory / f'{name}.partial.npy').replace(directory / f'{name}.npy')
    info = {'source': str(run), 'manifest_sha256': sha(run / 'input_manifest.json'), 'cases': cases,
            'frames': total, 'actions': actions, 'created_at': now(), 'prepare_binary_sha256': sha(directory / 'prepare_boards'),
            'checks': {'independent_replay_all_frames': True, 'all_teachers_legal': True, 'all_complete': True,
                       'held_out_excluded': True},
            'split_cases': {role: sum(c['role'] == role for c in cases) for role in ('train', 'validation')},
            'split_frames': {role: sum(c['frames'] for c in cases if c['role'] == role) for role in ('train', 'validation')}}
    assert info['split_frames'] == {'train': 886749, 'validation': 53337}
    info['array_sha256'] = {name: sha(directory / f'{name}.npy') for name in ARRAY_NAMES}
    save(directory / 'dataset.json', info)
    # 完成配列とそのハッシュを保存した後だけ、今回作った重複の生データを解放する。
    shutil.rmtree(directory / 'raw')
    status(run, 'data_ready', frames=total, actions=actions, split_frames=info['split_frames'])


class Dataset:
    def __init__(self, directory):
        self.run = directory; self.description = load(directory / 'dataset.json')
        self.source = Path(self.description['source'])
        for name in self.description['array_sha256']:
            setattr(self, name, np.load(directory / f'{name}.npy', mmap_mode='r'))
        self.cases = self.description['cases']; self.by_index = {c['index']: c for c in self.cases}
        self.splits = {role: np.concatenate([np.arange(c['frame_start'], c['frame_start'] + c['frames'])
                          for c in self.cases if c['role'] == role]) for role in ('train', 'validation')}
        self.lengths = np.zeros(self.static.shape[0], np.int64)
        self.train_case_count = self.description['split_cases']['train']
        self.maps, self.matrices, self.dirs = {}, {}, {}
        for case in self.cases:
            self.lengths[case['index']] = case['frames']
            if case['N'] not in self.maps:
                self.maps[case['N']], self.matrices[case['N']], self.dirs[case['N']] = d4_maps(case['N'])

    def batch(self, ids, epoch=0, groups=None):
        ids = np.asarray(ids, np.int64); cases = np.asarray(self.case_ids[ids], np.int64); B = len(ids)
        state = np.asarray(self.states[ids]); grid = np.arange(400)
        colors = (state[:, :, None] >> (4 * np.arange(8, dtype=np.uint32))) & 15
        present = colors != 0
        x = np.zeros((B, 400, 40), np.float32)
        x[:, :, :8] = self.static[cases]
        x[:, :, 4] = present.sum(-1) / 8.
        x[:, :, 5] = present.sum((1, 2))[:, None] / 256. * x[:, :, 0]
        layers = x[:, :, 8:].reshape(B, 400, 8, 4)
        layers[:, :, :, 0] = present
        layers[:, :, :, 1:] = self.layer[cases[:, None, None], colors, grid[None, :, None]]
        starts = self.offsets[ids]; counts = self.offsets[ids + 1] - starts
        length = int(counts.max()); valid = np.arange(length)[None] < counts[:, None]
        indexes = starts[:, None] + np.minimum(np.arange(length)[None], counts[:, None] - 1)
        codes = np.asarray(self.codes[indexes]); src = (codes & 511).astype(np.int64)
        d = (codes >> 12) & 3; l = ((codes >> 14) & 7) + 1
        dst = src + (DIRECTIONS[d, 0] * 20 + DIRECTIONS[d, 1]) * l.astype(np.int64)
        features = np.array(self.features[indexes], copy=True)
        if groups is None:
            groups = np.zeros(B, np.int64) if epoch == 0 else np.random.default_rng(SEED + epoch * 1000003 + int(ids[0])).integers(8, size=B)
        for b, group in enumerate(groups):
            if not group: continue
            N = self.by_index[int(cases[b])]['N']; mapping = self.maps[N][group]; matrix = self.matrices[N][group]
            old = x[b].copy(); dx = old[:, 9::4].copy(); dy = old[:, 10::4].copy()
            old[:, 9::4] = matrix[0, 0] * dx + matrix[0, 1] * dy
            old[:, 10::4] = matrix[1, 0] * dx + matrix[1, 1] * dy
            old[:, 2] = mapping // 20 / 20. * old[:, 0]; old[:, 3] = mapping % 20 / 20. * old[:, 0]
            x[b, mapping] = old
            src[b] = mapping[src[b]]; dst[b] = mapping[dst[b]]
            features[b, :, 4:8] = 0
            features[b, np.arange(length), 4 + self.dirs[N][group][d[b]]] = 1
        weight = len(self.splits['train']) / self.train_case_count / self.lengths[cases]
        return {'x': np.ascontiguousarray(x.transpose(0, 2, 1).reshape(B, 40, 20, 20)),
                'src': src, 'dst': dst.astype(np.int64), 'features': features, 'valid': valid,
                'target': np.asarray(self.targets[ids]), 'value': np.asarray(self.remaining[ids]),
                'weight': weight.astype(np.float32)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--run', type=Path, default=RUN)
    parser.add_argument('--limit', type=int, default=0, help='only convert the first N teachers; retain these for the full conversion')
    args = parser.parse_args(); prepare(args.run.resolve(), args.limit)
