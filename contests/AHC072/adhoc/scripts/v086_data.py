#!/usr/bin/env python3
"""v085の成功した初期個体集合を、探索を再実行せず学習配列へ変換する。"""
import argparse
from concurrent.futures import ProcessPoolExecutor, as_completed
import gzip
import json
from pathlib import Path
import subprocess
import time

import numpy as np

from v085_data import ROOT, move_hash, now, save, sha, status

SOURCE = ROOT / 'results/nn_rank/v085/20261002T144801_studio'
SEED, EPOCHS, BATCH = 86001, 60, 32


def examples(item):
    """窓の最後の実際の最良更新だけを教師にし、先行する同手数変更は使わない。"""
    original = (SOURCE / item['path']).read_text()
    assert sha(SOURCE / item['path']) == item['sha256']
    tokens = original.split()
    grid = tokens[2:2 + item['N']]
    cells = [c for row in grid for c in row if c != '#']
    ids = [i for i, c in enumerate(cells) if 'a' <= c <= 'l']
    assert len(ids) == item['M']
    positions = {value: i for i, value in enumerate(ids)}
    rows, seen, paths = [], set(), []
    for path in sorted((SOURCE / 'cases' / f"{item['index']:06d}").glob('*/round_*/search/episodes.jsonl.gz')):
        paths.append({'path': str(path.relative_to(SOURCE)), 'sha256': sha(path)})
        with gzip.open(path, 'rt') as stream:
            for episode in map(json.loads, stream):
                transitions = episode['transitions']
                action = transitions[-1]
                if action['kind'] not in ('regular', 'dependency'):
                    continue
                key = action['transition_id']
                assert key not in seen
                seen.add(key)
                before = transitions[-2]['after'] if len(transitions) > 1 else episode['start']
                assert move_hash(before) == action['before_hash'] and len(before) == action['before_T']
                gain = episode['best_before_T'] - episode['best_after_T']
                assert gain > 0 and action['after_T'] == episode['best_after_T'] < action['before_T']
                selected = sorted(action['selection']['ids'])
                assert 1 <= len(selected) <= 12 and len(set(selected)) == len(selected)
                assert set(selected) <= positions.keys()
                rows.append({'transition_id': key, 'before': before, 'selected': [positions[x] for x in selected],
                             'selected_ids': selected, 'kind': action['kind'], 'best_gain': gain,
                             'direct_gain': action['before_T'] - action['after_T'],
                             'teacher': episode['teacher'], 'before_hash': action['before_hash']})
    assert len(paths) == 6
    return original, ids, rows, paths


def extract_case(task):
    run, item, extractor, extractor_sha = task
    run, extractor = Path(run), Path(extractor)
    directory = run / 'cases' / f"{item['index']:06d}"
    directory.mkdir(parents=True, exist_ok=True)
    complete = directory / 'complete.json'
    if complete.exists():
        result = json.loads(complete.read_text())
        assert result['extractor_sha256'] == extractor_sha and result['input'] == item
        for key, digest in result['arrays'].items():
            assert sha(directory / f'{key}.npy') == digest
        return result
    original, ids, rows, sources = examples(item)
    count, M = len(rows), len(ids)
    if count:
        parts = [original.rstrip(), str(count)]
        for row in rows:
            parts.append(str(len(row['before'])))
            parts.extend(' '.join(map(str, move)) for move in row['before'])
        child = subprocess.run([str(extractor)], input=('\n'.join(parts) + '\n').encode(),
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=300, cwd=ROOT)
        (directory / 'extract.log').write_bytes(child.stderr)
        values = np.frombuffer(child.stdout, dtype=np.float32)
        width = M * 24 + 5 * M * M
        assert values.size == count * width and np.isfinite(values).all()
        packed = values.reshape(count, width)
        nodes = packed[:, :M * 24].reshape(count, M, 24).copy()
        edges = packed[:, M * 24:].reshape(count, 5, M, M).copy()
        assert (edges >= 0).all() and (edges <= 1.000001).all()
        row_sum = edges.sum(-1)
        assert np.all((abs(row_sum - 1) < 2e-6) | (row_sum == 0))
        assert not np.diagonal(edges, axis1=-2, axis2=-1).any()
    else:
        nodes, edges = np.zeros((0, M, 24), np.float32), np.zeros((0, 5, M, M), np.float32)
    targets = np.zeros((count, M), bool)
    weights = np.array([min(5, row['best_gain']) for row in rows], np.float32)
    if count:
        weights /= weights.sum()
    for i, row in enumerate(rows):
        targets[i, row['selected']] = True
    arrays = {'nodes': nodes, 'edges': edges, 'targets': targets, 'weights': weights,
              'item_ids': np.asarray(ids, np.int16)}
    hashes = {}
    for key, value in arrays.items():
        dest = directory / f'{key}.npy'
        with dest.with_suffix('.tmp').open('wb') as out:
            np.save(out, value)
        dest.with_suffix('.tmp').replace(dest)
        hashes[key] = sha(dest)
    save(directory / 'examples.json', [{k: v for k, v in row.items() if k != 'before'} for row in rows])
    result = {'input': item, 'examples': count, 'arrays': hashes, 'sources': sources,
              'extractor_sha256': extractor_sha, 'completed_at': now()}
    save(complete, result)
    return result


def prepare(run, extractor, workers=30, limit=None):
    run.mkdir(parents=True, exist_ok=True)
    items = json.loads((SOURCE / 'input_manifest.json').read_text())
    assert len(items) == 512 and sum(x['role'] == 'train' for x in items) == 384
    provenance = {'source': str(SOURCE), 'input_manifest_sha256': sha(SOURCE / 'input_manifest.json'),
                  'quality_sha256': sha(SOURCE / 'quality_512.json'), 'extractor_sha256': sha(extractor)}
    path = run / 'data_provenance.json'
    if path.exists():
        assert json.loads(path.read_text()) == provenance
    else:
        save(path, provenance)
    tasks = [(str(run), item, str(extractor), provenance['extractor_sha256']) for item in items[:limit]]
    started = time.monotonic()
    results = []
    with ProcessPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(extract_case, task): task[1]['index'] for task in tasks}
        for future in as_completed(futures):
            result = future.result()
            assert result['input']['index'] == futures[future]
            results.append(result)
            if len(results) % 32 == 0 or len(results) == len(tasks):
                status(run, 'extracting_success_graphs', inputs=len(results), total=len(tasks),
                       elapsed_sec=time.monotonic() - started)
    if limit is not None:
        save(run / 'data_calibration.json', {'inputs': len(results), 'elapsed_sec': time.monotonic() - started})
        return
    results.sort(key=lambda r: r['input']['index'])
    counts = {role: sum(r['examples'] for r in results if r['input']['role'] == role) for role in ('train', 'validation')}
    assert counts == {'train': 10059, 'validation': 2843}, counts
    contributing = {role: sum(r['examples'] > 0 for r in results if r['input']['role'] == role) for role in counts}
    assert contributing == {'train': 376, 'validation': 124}
    total, squares, observations = np.zeros(24), np.zeros(24), 0
    for result in results:
        if result['input']['role'] != 'train':
            continue
        nodes = np.load(run / 'cases' / f"{result['input']['index']:06d}" / 'nodes.npy', mmap_mode='r')
        if nodes.size:
            total += nodes.sum((0, 1), dtype=np.float64)
            squares += np.square(nodes.astype(np.float64)).sum((0, 1))
            observations += nodes.shape[0] * nodes.shape[1]
    mean = total / observations
    scale = np.sqrt(np.maximum(squares / observations - mean * mean, 0))
    scale[scale < 1e-6] = 1
    result = {**provenance, 'cases': results, 'counts': counts, 'contributing_inputs': contributing,
              'empty_input_indices': [r['input']['index'] for r in results if not r['examples']],
              'mean': mean.tolist(), 'scale': scale.tolist(), 'training_node_observations': observations,
              'completed_at': now()}
    save(run / 'dataset.json', result)
    status(run, 'data_ready', counts=counts, contributing_inputs=contributing)


class Dataset:
    def __init__(self, run):
        self.run = run
        self.description = json.loads((run / 'dataset.json').read_text())
        self.mean = np.asarray(self.description['mean'], np.float32)
        self.scale = np.asarray(self.description['scale'], np.float32)
        self.arrays, self.rows, self.splits = {}, [], {'train': [], 'validation': []}
        for case in self.description['cases']:
            item = case['input']; index = item['index']; folder = run / 'cases' / f'{index:06d}'
            self.arrays[index] = {key: np.load(folder / f'{key}.npy', mmap_mode='r')
                                  for key in ('nodes', 'edges', 'targets', 'weights')}
            for row in range(case['examples']):
                self.splits[item['role']].append(len(self.rows))
                self.rows.append((index, row, item['M'], item['role']))

    def batch(self, indices, epoch=0):
        rows = [self.rows[i] for i in indices]
        B, M = len(rows), max(row[2] for row in rows)
        node = np.zeros((B, M, 24), np.float32)
        edge = np.zeros((B, 5, M, M), np.float32)
        valid = np.zeros((B, M), bool)
        target = np.zeros((B, M), bool)
        prefix = np.zeros((B, 13, M), np.float32)
        weight = np.empty(B, np.float32)
        sizes = np.empty(B, np.int64)
        case_indices = np.empty(B, np.int64)
        for b, (global_id, (index, row, n, role)) in enumerate(zip(indices, rows)):
            arrays = self.arrays[index]
            node[b, :n] = (arrays['nodes'][row] - self.mean) / self.scale
            edge[b, :, :n, :n] = arrays['edges'][row]
            valid[b, :n] = True; target[b, :n] = arrays['targets'][row]
            selected = np.flatnonzero(target[b])
            # 同じepoch/例なら中断位置に関係なく同じ部分集合を使う。
            order = np.random.default_rng(np.random.SeedSequence([SEED, epoch, int(global_id)])).permutation(selected)
            sizes[b] = len(selected); weight[b] = arrays['weights'][row]; case_indices[b] = index
            for k, at in enumerate(order):
                prefix[b, k + 1:] = prefix[b, k]
                prefix[b, k + 1:, at] = 1
        steps = int(sizes.max()) + 1
        return {'nodes': node, 'edges': edge, 'valid': valid, 'target': target,
                'prefix': prefix[:, :steps], 'weights': weight, 'sizes': sizes, 'cases': case_indices}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--run', type=Path, required=True)
    parser.add_argument('--extractor', type=Path, required=True)
    parser.add_argument('--workers', type=int, default=30)
    parser.add_argument('--limit', type=int)
    args = parser.parse_args()
    prepare(args.run.resolve(), args.extractor.resolve(), args.workers, args.limit)
