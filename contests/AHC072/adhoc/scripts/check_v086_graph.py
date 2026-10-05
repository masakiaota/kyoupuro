#!/usr/bin/env python3
"""v086の個体・関係特徴をPythonの独立再生と照合する。"""
import argparse
from collections import deque
import json
from pathlib import Path
import subprocess

import numpy as np

from check_v084_set import reference as node_reference, self_test
from check_v082_relation import replay
from v086_data import SOURCE, ROOT, examples, save, sha


def relation_reference(text, moves):
    lines = text.splitlines(); N, K = map(int, lines[0].split()); grid = lines[1:N+1]
    cells = [(r, c) for r in range(N) for c in range(N) if grid[r][c] != '#']; floors = set(cells)
    ids = [i for i, (r, c) in enumerate(cells) if grid[r][c].islower()]; M = len(ids)
    positions = {identity: i for i, identity in enumerate(ids)}
    colors = [grid[cells[i][0]][cells[i][1]] for i in ids]
    edges = np.zeros((5, M, M), np.float64)
    for i, identity in enumerate(ids):
        start = cells[identity]; queue = deque([start]); distances = {start: 0}
        while queue:
            r, c = queue.popleft()
            for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                p = (r+dr, c+dc)
                if p in floors and p not in distances:
                    distances[p] = distances[r, c] + 1; queue.append(p)
        for j, other in enumerate(ids):
            if i != j:
                edges[0, i, j] = colors[i] == colors[j]
                edges[1, i, j] = 1 / (1 + distances[cells[other]])
    record = replay(text, {'current': moves})
    for _, length, _, passengers, supporters in record['events']:
        for a in passengers:
            for b in passengers:
                if a != b:
                    edges[2, positions[a], positions[b]] += 1
        if length > 1:
            for a in supporters:
                for b in passengers:
                    edges[3, positions[a], positions[b]] += 1
                    edges[4, positions[b], positions[a]] += 1
    total = edges.sum(-1, keepdims=True)
    edges /= np.where(total > 0, total, 1)
    return ids, edges, record


def artificial():
    result = self_test()
    text = '5 1\n.....\n.aaa.\n.a...\n.....\n....A\n'
    moves = [[7,0,2,1], [8,0,2,1], [7,0,2,1], [11,0,0,1], [6,2,3,3]]
    ids, edges, _ = relation_reference(text, moves)
    assert ids == [6,7,8,11]
    assert edges[3,0,2] == edges[3,0,3] == .5
    assert edges[4,2,0] == edges[4,2,1] == .5
    assert edges[3,2].sum() == edges[4,0].sum() == 0
    assert edges[2,2,3] == edges[2,3,2] == 1
    assert not np.diagonal(edges, axis1=1, axis2=2).any()
    return {**result, 'support_direction': True, 'zero_rows': True, 'copassengers': True}


def check(extractor, output):
    result = {'passed': False, 'artificial': artificial(), 'groups': 0, 'max_node_error': 0.,
              'max_edge_error': 0., 'extractor_sha256': sha(extractor)}
    for item in json.loads((SOURCE / 'input_manifest.json').read_text())[:4]:
        text, ids, rows, _ = examples(item)
        selected = rows[:1] + rows[-1:] if len(rows) > 1 else rows
        parts = [text.rstrip(), str(len(selected))]
        for row in selected:
            parts.append(str(len(row['before'])))
            parts.extend(' '.join(map(str, m)) for m in row['before'])
        child = subprocess.run([str(extractor)], input=('\n'.join(parts)+'\n').encode(),
                               capture_output=True, check=True, cwd=ROOT, timeout=60)
        M = len(ids); width = M * 24 + 5 * M * M
        actual = np.frombuffer(child.stdout, np.float32).reshape(len(selected), width)
        for values, row in zip(actual, selected):
            expected_ids, nodes = node_reference(text, {'current': row['before']})
            graph_ids, edges, record = relation_reference(text, row['before'])
            assert ids == expected_ids == graph_ids and not any(record['remaining'].values())
            global_features = np.tile([item['N']/20, item['M']/100, item['K']/10, len(row['before'])/400], (M, 1))
            nodes = np.concatenate((nodes, global_features), axis=1)
            got_node = values[:M*24].reshape(M,24); got_edge = values[M*24:].reshape(5,M,M)
            np.testing.assert_allclose(got_node, nodes, rtol=1e-6, atol=1e-7)
            np.testing.assert_allclose(got_edge, edges, rtol=1e-6, atol=1e-7)
            result['groups'] += 1
            result['max_node_error'] = max(result['max_node_error'], float(abs(got_node-nodes).max()))
            result['max_edge_error'] = max(result['max_edge_error'], float(abs(got_edge-edges).max()))
    assert result['groups'] == 8
    result['passed'] = True
    save(output, result)
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--extractor', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True); args=parser.parse_args()
    check(args.extractor.resolve(), args.output.resolve())
