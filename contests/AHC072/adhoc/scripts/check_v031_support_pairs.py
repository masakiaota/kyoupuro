#!/usr/bin/env python3
"""Independent support-mark checks and replay of frozen reconstruction outputs."""
from collections import Counter, deque
import csv
import hashlib
import json
from pathlib import Path
from statistics import mean

from analyze_v026_late_starts import saved_moves
from check_v028_two_orders import validate

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v031_audit'
DIRECTIONS = {'U': (-1, 0), 'D': (1, 0), 'L': (0, -1), 'R': (0, 1)}


def replay(case):
    lines = (ROOT / 'tools/in' / case).read_text().splitlines()
    n, _ = map(int, lines[0].split())
    grid = ''.join(lines[1:n+1])
    floor = [p for p, c in enumerate(grid) if c != '#']
    index = {p: i for i, p in enumerate(floor)}
    colors = {index[p]: ord(c)-ord('a') for p, c in enumerate(grid) if 'a' <= c <= 'z'}
    nests = {index[p]: ord(c)-ord('A') for p, c in enumerate(grid) if 'A' <= c <= 'Z'}
    towers = {p: [p] for p in colors}
    states, uses, origins = [], [], []
    for t, (row, col, keep, direction, length) in enumerate(saved_moves(case)):
        p = index[row*n+col]
        states.append({at: tuple(ids) for at, ids in towers.items() if ids})
        origins.append(p)
        dr, dc = DIRECTIONS[direction]
        q = index[(row+dr*length)*n+col+dc*length]
        if length > 1:
            uses.append((t, set(towers[p][:keep]), set(towers[p][keep:]), keep+1-length))
        flying = towers[p][keep:]
        towers[p] = towers[p][:keep]
        towers.setdefault(q, []).extend(reversed(flying))
        for at in (p, q):
            while towers[at] and colors[towers[at][-1]] == nests.get(at, -1):
                towers[at].pop()
    assert not any(towers.values())
    adjacent = {}
    for p, absolute in enumerate(floor):
        row, col = divmod(absolute, n)
        adjacent[p] = [index[(row+dr)*n+col+dc] for dr, dc in DIRECTIONS.values()
                       if 0 <= row+dr < n and 0 <= col+dc < n and (row+dr)*n+col+dc in index]
    distances = {}
    for p in set(origins):
        distances[p] = {p: 0}
        queue = deque([p])
        while queue:
            at = queue.popleft()
            for q in adjacent[at]:
                if q not in distances[p]:
                    distances[p][q] = distances[p][at]+1
                    queue.append(q)
    return colors, nests, states, uses, origins, distances


def check_marks():
    cases, total = {}, Counter()
    current = None
    for line in (OUT / 'marks.jsonl').open():
        item = json.loads(line)
        case = item['case']
        if case != current:
            current = case
            colors, nests, states, uses, origins, distances = replay(case)
        time, p, keep, limit = (item[k] for k in ('time', 'p', 'keep', 'limit'))
        assert p == origins[time]
        state = states[time]
        removed = set(state[p][keep:])
        assert 1 <= len(removed) < limit
        assert keep == 0 or colors[state[p][keep-1]] != nests.get(p, -1)
        count = cases.setdefault(case, Counter())
        count['queries'] += 1
        expected = {}
        for q, ids in state.items():
            if q == p or distances[p][q] > 10:
                continue
            lower_bound = max(0, len(ids)-(limit-len(removed)))
            cut = next((k for k in range(lower_bound, len(ids))
                        if k == 0 or colors[ids[k-1]] != nests.get(q, -1)), None)
            if cut is None:
                continue
            ranks = {token: rank for rank, token in enumerate(ids)}
            expected_keep = -1
            for use_time, lower, flight, slack in uses:
                if use_time < time or len(lower & removed) <= slack:
                    continue
                surviving = flight-removed
                if surviving and surviving <= set(ids[cut:]):
                    expected_keep = max(expected_keep, min(ranks[token] for token in surviving))
            expected[q] = [cut, expected_keep]
        assert {q: [k, mark] for q, k, mark in item['partners']} == expected, (case, time, p, keep, limit)
        related = sum(mark >= 0 for _, mark in expected.values())
        assert related == item['related']
        count['available'] += related > 0
        count['partners'] += len(expected)
        count['related_partners'] += related
    assert len(cases) == 100
    for count in cases.values():
        total.update(count)
    assert total['related_partners'] > 0
    return {'verified_cases': len(cases), 'total': dict(total), 'cases_with_related': sum(c['available'] > 0 for c in cases.values())}


def check_calls():
    variants, tables = {}, {}
    for variant in ('parent', 'child'):
        with (OUT / f'{variant}.csv').open() as f:
            rows = list(csv.DictReader(f))
        for row in rows:
            for key in row.keys()-{'case', 'bank_hash', 'raw_us', 'polish_us', 'mark_us'}:
                row[key] = int(row[key])
            for key in ('raw_us', 'polish_us', 'mark_us'):
                row[key] = float(row[key])
        by_key = {(r['case'], r['sample']): r for r in rows}
        assert len(by_key) == len(rows) == 6400
        cases = Counter()
        for line in (OUT / f'{variant}_moves.jsonl').open():
            sample = json.loads(line)
            row = by_key.pop((sample['case'], sample['sample']))
            cases[sample['case']] += 1
            for kind in ('raw', 'polished'):
                moves = sample[kind]
                assert bool(row['ok']) == (moves is not None)
                assert row[kind+'_T'] == (len(moves) if moves is not None else -1)
                if moves is not None:
                    validate(sample['case'], moves)
            if row['ok']:
                assert row['polished_T'] <= row['raw_T'] <= row['original_T']+3
                assert row['duplicate'] == (sample['polished'] == saved_moves(sample['case']))
            if variant == 'child':
                assert row['related_completed'] == int(row['ok'] and row['related_selected'])
        assert not by_key and len(cases) == 100 and set(cases.values()) == {64}
        tables[variant] = {(r['case'], r['sample']): r for r in rows}
        variants[variant] = {'all_100': describe(rows), 'generated_99': describe([r for r in rows if r['case'] != '0000.txt'])}
    matching_banks = all(tables['parent'][key]['bank_hash'] == row['bank_hash'] for key, row in tables['child'].items())
    assert matching_banks, 'Start-time bank order differs between variants'
    pairs = Counter()
    for key, child in tables['child'].items():
        parent = tables['parent'][key]
        if key[0] == '0000.txt':
            continue
        pairs['both_completed'] += bool(parent['ok'] and child['ok'])
        pairs['child_only_completed'] += bool(child['ok'] and not parent['ok'])
        pairs['parent_only_completed'] += bool(parent['ok'] and not child['ok'])
        # Failed calls and worse candidates leave the frozen starting plan unchanged.
        p = min(parent['original_T'], parent['polished_T']) if parent['ok'] else parent['original_T']
        c = min(child['original_T'], child['polished_T']) if child['ok'] else child['original_T']
        pairs['child_better'] += c < p
        pairs['parent_better'] += p < c
        pairs['equal'] += c == p
        pairs['delta_sum_overlapping'] += c-p
    child = variants['child']['all_100']
    assert child['related_selected'] > 0 and child['related_completed'] > 0
    return {'verified_cases': 100, 'same_start_time_banks': matching_banks,
            'variants': variants, 'matched_generated_99': dict(pairs)}


def describe(rows):
    completed = [r for r in rows if r['ok']]
    improved = [r for r in completed if r['polished_T'] < r['original_T']]
    return {'calls': len(rows), 'completed': len(completed), 'improved': len(improved),
            'improved_cases': len({r['case'] for r in improved}),
            'gain_sum_overlapping': sum(r['original_T']-r['polished_T'] for r in improved),
            'duplicates': sum(r['duplicate'] for r in completed),
            'mean_raw_us': mean(r['raw_us'] for r in rows),
            'mean_polish_us': mean(r['polish_us'] for r in rows),
            'mean_mark_us': mean(r['mark_us'] for r in rows),
            'related_selected': sum(max(0, r['related_selected']) for r in rows),
            'related_completed': sum(max(0, r['related_completed']) for r in rows),
            'related_improved': sum(r['related_selected'] == 1 for r in improved)}


def main():
    audit = json.loads((OUT / 'static_verification.json').read_text())
    for name, key in (('v031_support_pair_fast_math', 'solver_sha256'), ('v026_late_start_lns', 'parent_sha256')):
        assert hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    for directory, key in (('tools/in', 'input_sha256'), ('results/out/v026_late_start_lns', 'saved_v026_output_sha256')):
        for name, digest in audit[key].items():
            assert hashlib.sha256((ROOT / directory / name).read_bytes()).hexdigest() == digest
    result = {'solver_sha256': audit['solver_sha256'], 'marks': check_marks(), 'reconstruction': check_calls()}
    result['mechanism_passed'] = True
    (OUT / 'diagnostic_check.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
