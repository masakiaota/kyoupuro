#!/usr/bin/env python3
"""v089の完成列を実盤面と合法操作へ変換する。分割は入力単位で固定する。"""
import argparse
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'results/nn_rank/v085/20261002T144801_studio'
RUN = ROOT / 'results/nn_rank/v089/20261003_board_studio'
SEED, BATCH, EPOCHS = 89001, 64, 40
DIRECTIONS = np.array([[-1, 0], [1, 0], [0, -1], [0, 1]], dtype=np.int32)


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix('.tmp')
    tmp.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    tmp.replace(path)


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        while block := f.read(1 << 20):
            h.update(block)
    return h.hexdigest()


def status(run, stage, **kwargs):
    row = dict(stage=stage, updated_at=now(), **kwargs)
    save(run / 'status.json', row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


class Geometry:
    """C++から独立した盤面再生と、巣までの床上距離。"""
    def __init__(self, path):
        lines = path.read_text().splitlines()
        self.N, self.K = map(int, lines[0].split())
        self.C = lines[1:self.N + 1]
        self.floor = np.zeros(400, bool)
        self.nest = np.full(400, -1, np.int32)
        self.initial = np.zeros(400, np.uint32)
        for i, row in enumerate(self.C):
            for j, ch in enumerate(row):
                f = i * 20 + j
                self.floor[f] = ch != '#'
                if ch.isupper(): self.nest[f] = ord(ch) - 65
                if ch.islower(): self.initial[f] = ord(ch) - 96
        self.distance = np.zeros((13, 400), np.float32)
        self.goal = np.zeros((13, 2), np.int32)
        for c in range(self.K):
            goal, = np.flatnonzero(self.nest == c)
            self.goal[c + 1] = divmod(int(goal), 20)
            d = {int(goal): 0}; queue = deque([int(goal)])
            while queue:
                p = queue.popleft(); i, j = divmod(p, 20)
                for di, dj in DIRECTIONS:
                    ni, nj = i + int(di), j + int(dj)
                    if 0 <= ni < self.N and 0 <= nj < self.N:
                        q = ni * 20 + nj
                        if self.floor[q] and q not in d:
                            d[q] = d[p] + 1; queue.append(q)
            assert len(d) == int(self.floor.sum())
            for p, value in d.items(): self.distance[c + 1, p] = value

    def apply(self, state, line):
        i, j, k, direction, length = line.split()
        i, j, k, length = map(int, (i, j, k, length))
        d = 'UDLR'.index(direction); p = i * 20 + j
        assert 0 <= i < self.N and 0 <= j < self.N and self.floor[p]
        a = unpack(int(state[p])); assert 0 <= k < len(a) and 1 <= length <= k + 1
        di, dj = map(int, DIRECTIONS[d])
        for step in range(1, length + 1):
            ni, nj = i + di * step, j + dj * step
            assert 0 <= ni < self.N and 0 <= nj < self.N and self.floor[ni * 20 + nj]
        q = (i + di * length) * 20 + j + dj * length
        b = unpack(int(state[q])); assert len(b) + len(a) - k <= 8
        a, b = a[:k], b + list(reversed(a[k:]))
        for cell, tower in ((p, a), (q, b)):
            while tower and tower[-1] - 1 == self.nest[cell]: tower.pop()
            state[cell] = sum(int(c) << (4 * t) for t, c in enumerate(tower))
        return p | (k << 9) | (d << 12) | ((length - 1) << 14)


def unpack(bits):
    a = []
    while bits:
        a.append(bits & 15); bits >>= 4
    return a


def remaining(state):
    return int(np.count_nonzero((state[:, None] >> (4 * np.arange(8, dtype=np.uint32))) & 15))


def case_prepare(run, item):
    idx = item['index']; directory = run / 'raw' / f'{idx:06d}'
    input_path = SOURCE / item['path']; solution = SOURCE / 'cases' / f'{idx:06d}' / 'best.txt'
    assert sha(input_path) == item['sha256']
    record = json.loads((solution.parent / 'complete.json').read_text())
    assert sha(solution) == record['best_sha256']
    result = subprocess.run([ROOT / 'target/release/prepare_v089_boards', input_path, solution, directory],
                            check=True, capture_output=True, text=True)
    sizes = json.loads(result.stdout)
    T, A = sizes['frames'], sizes['actions']
    states = np.memmap(directory / 'states.raw', mode='r', dtype='<u4', shape=(T, 400))
    offsets = np.fromfile(directory / 'offsets.raw', dtype='<u8')
    targets = np.fromfile(directory / 'targets.raw', dtype='<i4')
    codes = np.memmap(directory / 'codes.raw', mode='r', dtype='<u4', shape=(A,))
    geometry = Geometry(input_path); state = geometry.initial.copy()
    lines = [x for x in solution.read_text().splitlines() if x.strip()]
    assert len(lines) == T == record['T'] and offsets[-1] == A and len(offsets) == T + 1
    for t, line in enumerate(lines):
        assert np.array_equal(state, states[t]), (idx, t, 'state disagreement')
        actual = geometry.apply(state, line)
        assert codes[int(offsets[t]) + int(targets[t])] == actual
    assert remaining(state) == 0
    return dict(item, frames=T, actions=A, solution_sha256=sha(solution), baseline_T=record['baseline_T'])


def prepare(run):
    run.mkdir(parents=True, exist_ok=True)
    if (run / 'dataset.json').exists():
        raise RuntimeError('dataset already exists; use the saved dataset')
    items = json.loads((SOURCE / 'input_manifest.json').read_text())
    assert len(items) == 512
    with ThreadPoolExecutor(max_workers=20) as pool:
        cases = list(pool.map(lambda item: case_prepare(run, item), items))
    total = sum(x['frames'] for x in cases); actions = sum(x['actions'] for x in cases)
    arrays = {}
    for name, dtype, shape in [('states', 'uint32', (total, 400)), ('offsets', 'int64', (total + 1,)),
                                ('targets', 'int64', (total,)), ('codes', 'uint32', (actions,)),
                                ('features', 'float32', (actions, 16)), ('case_ids', 'int16', (total,)),
                                ('remaining', 'float32', (total,))]:
        arrays[name] = np.lib.format.open_memmap(run / f'{name}.npy', mode='w+', dtype=dtype, shape=shape)
    frame_base = action_base = 0
    for item in cases:
        T, A = item['frames'], item['actions']; directory = run / 'raw' / f"{item['index']:06d}"
        item['frame_start'] = frame_base
        for name, dtype, shape in [('states', '<u4', (T, 400)), ('targets', '<i4', (T,)),
                                    ('codes', '<u4', (A,)), ('features', '<f4', (A, 16))]:
            src = np.memmap(directory / f'{name}.raw', mode='r', dtype=dtype, shape=shape)
            base = action_base if name in ('codes', 'features') else frame_base
            arrays[name][base:base + shape[0]] = src
        arrays['offsets'][frame_base:frame_base + T + 1] = np.fromfile(directory / 'offsets.raw', dtype='<u8') + action_base
        arrays['case_ids'][frame_base:frame_base + T] = item['index']
        arrays['remaining'][frame_base:frame_base + T] = np.arange(T, 0, -1)
        frame_base += T; action_base += A
    for a in arrays.values(): a.flush()
    info = {'source': str(SOURCE), 'manifest_sha256': sha(SOURCE / 'input_manifest.json'), 'cases': cases,
            'frames': total, 'actions': actions, 'created_at': now(),
            'checks': {'independent_replay_all_frames': True, 'all_teachers_legal': True, 'all_complete': True},
            'split_frames': {role: sum(x['frames'] for x in cases if x['role'] == role) for role in ('train', 'validation')}}
    assert info['split_frames'] == dict(train=83331, validation=26023)
    info['array_sha256'] = {name: sha(run / f'{name}.npy') for name in arrays}
    save(run / 'dataset.json', info)
    status(run, 'data_ready', frames=total, actions=actions, split_frames=info['split_frames'])


def d4_maps(N):
    """先に左右反転、次に反時計回り回転。盤外の0領域は移動しない。"""
    mappings, matrices, directions = [], [], []
    for group in range(8):
        matrix = np.array([[1, 0], [0, -1 if group >= 4 else 1]], dtype=np.int32)
        for _ in range(group % 4): matrix = np.array([[0, -1], [1, 0]]) @ matrix
        mapping = np.arange(400)
        for i in range(N):
            for j in range(N):
                ni, nj = i, N - 1 - j if group >= 4 else j
                for _ in range(group % 4): ni, nj = N - 1 - nj, ni
                mapping[i * 20 + j] = ni * 20 + nj
        dirs = [next(k for k, v in enumerate(DIRECTIONS) if np.array_equal(v, matrix @ d)) for d in DIRECTIONS]
        mappings.append(mapping); matrices.append(matrix); directions.append(dirs)
    return np.array(mappings), np.array(matrices), np.array(directions)


class Dataset:
    def __init__(self, run):
        self.run = run; self.description = json.loads((run / 'dataset.json').read_text())
        for name in self.description['array_sha256']:
            setattr(self, name, np.load(run / f'{name}.npy', mmap_mode='r'))
        self.cases = self.description['cases']
        self.splits = {role: np.concatenate([np.arange(c['frame_start'], c['frame_start'] + c['frames'])
                          for c in self.cases if c['role'] == role]) for role in ('train', 'validation')}
        self.static = np.zeros((512, 400, 8), np.float32)
        self.layer = np.zeros((512, 13, 400, 3), np.float32)
        self.lengths = np.array([c['frames'] for c in self.cases])
        self.maps, self.matrices, self.dirs = {}, {}, {}
        rows, cols = np.divmod(np.arange(400), 20)
        for case in self.cases:
            idx = case['index']; geo = Geometry(SOURCE / case['path']); floor = geo.floor
            x = self.static[idx]
            x[:, 0] = floor; x[:, 1] = geo.nest >= 0
            x[:, 2] = rows / 20.; x[:, 3] = cols / 20.; x[:, 6] = geo.N / 20.; x[:, 7] = geo.K / 12.
            x[~floor] = 0
            for c in range(1, geo.K + 1):
                self.layer[idx, c, :, 0] = (geo.goal[c, 0] - rows) / 20.
                self.layer[idx, c, :, 1] = (geo.goal[c, 1] - cols) / 20.
                self.layer[idx, c, :, 2] = geo.distance[c] / 40.
            if geo.N not in self.maps:
                self.maps[geo.N], self.matrices[geo.N], self.dirs[geo.N] = d4_maps(geo.N)

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
            N = self.cases[cases[b]]['N']; mapping = self.maps[N][group]; matrix = self.matrices[N][group]
            old = x[b].copy(); dx = old[:, 9::4].copy(); dy = old[:, 10::4].copy()
            old[:, 9::4] = matrix[0, 0] * dx + matrix[0, 1] * dy
            old[:, 10::4] = matrix[1, 0] * dx + matrix[1, 1] * dy
            old[:, 2] = mapping // 20 / 20. * old[:, 0]; old[:, 3] = mapping % 20 / 20. * old[:, 0]
            x[b, mapping] = old
            src[b] = mapping[src[b]]; dst[b] = mapping[dst[b]]
            features[b, :, 4:8] = 0
            features[b, np.arange(length), 4 + self.dirs[N][group][d[b]]] = 1
        weight = len(self.splits['train']) / 384. / self.lengths[cases]
        return {'x': np.ascontiguousarray(x.transpose(0, 2, 1).reshape(B, 40, 20, 20)),
                'src': src, 'dst': dst.astype(np.int64), 'features': features, 'valid': valid,
                'target': np.asarray(self.targets[ids]), 'value': np.asarray(self.remaining[ids]),
                'weight': weight.astype(np.float32)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--run', type=Path, default=RUN)
    args = parser.parse_args(); prepare(args.run.resolve())
