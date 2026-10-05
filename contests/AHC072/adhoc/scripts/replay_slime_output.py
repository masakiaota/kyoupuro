#!/usr/bin/env python3
"""Validate and describe an existing move sequence. Does not generate moves."""

import argparse
from collections import Counter, deque
import json
from pathlib import Path


def replay(input_path, output_path):
    lines = input_path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = lines[1:N + 1]
    tower, nest = {}, {}
    for i in range(N):
        for j, ch in enumerate(C[i]):
            if ch != '#':
                tower[i, j] = [ch] if ch.islower() else []
            if ch.isupper():
                nest[i, j] = ch.lower()
    distance = {}
    directions = {'U': (-1, 0), 'D': (1, 0), 'L': (0, -1), 'R': (0, 1)}
    for cell, color in nest.items():
        d = {cell: 0}
        queue = deque([cell])
        while queue:
            i, j = queue.popleft()
            for di, dj in directions.values():
                q = i + di, j + dj
                if q in tower and q not in d:
                    d[q] = d[i, j] + 1
                    queue.append(q)
        distance[color] = d
    initial = sum(len(a) for a in tower.values())
    shortest = sum(distance[c][p] for p, a in tower.items() for c in a)
    metrics = Counter(M=initial, shortest_distance_sum=shortest,
                      max_height=max(map(len, tower.values())))
    per_color = {chr(97+c): Counter() for c in range(K)}
    operations = []
    for index, line in enumerate(output_path.read_text().splitlines()):
        if not line.strip():
            continue
        i, j, k, d, l = line.split()
        i, j, k, l = map(int, (i, j, k, l))
        p = i, j
        assert p in tower and 0 <= k < len(tower[p]), (index, line, 'source')
        assert 1 <= l <= k+1 and d in directions, (index, line, 'jump')
        di, dj = directions[d]
        for step in range(1, l+1):
            assert (i+di*step, j+dj*step) in tower, (index, line, 'wall')
        q = i+di*l, j+dj*l
        moving = tower[p][k:]
        assert len(tower[q]) + len(moving) <= 8, (index, line, 'height')
        before = {str(p): ''.join(tower[p]), str(q): ''.join(tower[q])}
        metrics['T'] += 1
        metrics['W'] += len(moving)*l
        metrics['moved_count'] += len(moving)
        metrics['mixed_moves'] += len(set(moving)) > 1
        metrics['long_jumps'] += l > 1
        metrics['single_moves'] += len(moving) == 1
        metrics['max_packet'] = max(metrics['max_packet'], len(moving))
        for c in moving:
            per_color[c]['W'] += l
            per_color[c]['toward_home'] += distance[c][p] - distance[c][q]
        tower[p] = tower[p][:k]
        tower[q].extend(reversed(moving))
        metrics['max_height'] = max(metrics['max_height'], len(tower[q]))
        returned = {'source': '', 'destination': ''}
        for cell, side in ((p, 'source'), (q, 'destination')):
            while tower[cell] and tower[cell][-1] == nest.get(cell):
                returned[side] += tower[cell].pop()
            metrics[f'{side}_returned'] += len(returned[side])
        operations.append({'turn': len(operations)+1, 'action': line,
                           'source': p, 'destination': q,
                           'before': before, 'moving_bottom_to_top': ''.join(moving),
                           'after': {str(p): ''.join(tower[p]), str(q): ''.join(tower[q])},
                           'returned': returned})
    metrics['E'] = sum(map(len, tower.values()))
    metrics['S'] = metrics['T'] + 100000*metrics['E']
    return {'input': str(input_path), 'output': str(output_path),
            'metrics': dict(metrics), 'per_color': per_color,
            'operations': operations}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--json', type=Path)
    args = parser.parse_args()
    report = replay(args.input, args.output)
    if args.json:
        args.json.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'operations'},
                     ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
