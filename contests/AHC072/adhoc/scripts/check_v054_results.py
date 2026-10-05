#!/usr/bin/env python3
"""Replay saved v054 results independently; never execute a solution program."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from analyze_v047_coordination import inspect
from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v054_audit'
BIN = 'v054_route_slack'
PARENT = 'v050_joint_towers'
PARENT_RUN = '20260929T104058+0900_v050_joint_towers_71ae82'
LONG = ROOT / 'results/long_search/v047/20260929T005348_ca87d7da'
DIRS = {'U': (-1, 0), 'D': (1, 0), 'L': (0, -1), 'R': (0, 1)}


def verify_sources():
    audit = json.loads((AUDIT / 'static_verification.json').read_text())
    for name, key in ((BIN, 'solver_sha256'), (PARENT, 'parent_sha256')):
        assert hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    assert hashlib.sha256((ROOT / 'adhoc/bin/check_v054_route_slack.cpp').read_bytes()).hexdigest() == audit['helper_sha256']
    assert hashlib.sha256((ROOT / 'src/bin/v049_coupled_routes.cpp').read_bytes()).hexdigest() == audit['route_parent_sha256']
    for case, digest in audit['input_sha256'].items():
        assert hashlib.sha256((ROOT / 'tools/in' / case).read_bytes()).hexdigest() == digest
    for path, digest in audit['plan_sha256'].items():
        assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest
    old = (ROOT / 'src/bin/v049_coupled_routes.cpp').read_text()
    strict = old[old.index('int coupleTransferWindow('):old.index('// Between two cells without home returns,')]
    strict = strict.replace('coupleTransferWindow', 'strictTransferWindow').replace('polishCoupledRoutes', 'strictPolishCoupledRoutes')
    assert strict in (ROOT / 'adhoc/bin/check_v054_route_slack.cpp').read_text()
    return audit


def plan_path(row):
    if row['group'] == 'parent':
        return ROOT / 'results/out' / PARENT / (row['case'] + '.txt')
    if row['group'] == 'initial':
        return LONG / 'cases' / row['case'] / 'seeds/00.txt'
    return LONG / 'cases' / row['case'] / 'continuous/round_0000/search/before' / (row['record'] + '.txt')


def verify_patches():
    patches = [json.loads(s) for s in (AUDIT / 'patches.jsonl').read_text().splitlines()]
    reports = {}
    for patch in patches:
        key = (patch['group'], patch['case'], patch['record'])
        if key not in reports:
            reports[key] = inspect(ROOT / 'tools/in' / (patch['case'] + '.txt'), plan_path(patch))
        r = reports[key]
        board = dict(zip(r['floor'], r['states'][patch['begin']]))
        a, b = patch['begin'], 0
        peak = 0
        relaxed = False
        assert 2 <= len(patch['boundaries']) <= 8 and patch['end'] - a <= 64
        assert patch['before'] == [[int(x) if i != 3 else x for i, x in enumerate(op['action'].split())]
                                   for op in r['operations'][a:patch['end']]]
        for next_a, next_b in patch['boundaries']:
            if b < next_b:
                assert patch['after'][b][:3] == patch['before'][a - patch['begin']][:3]
            q = tuple(patch['before'][a - patch['begin']][:2])
            relaxed |= (next_b - b >= next_a - a) if a == patch['begin'] else (next_b - b > next_a - a)
            while b < next_b:
                i, j, k, d, length = patch['after'][b]
                p = i, j
                assert p in board and 0 <= k < len(board[p]) and 1 <= length <= k + 1
                di, dj = DIRS[d]
                for step in range(1, length + 1):
                    q = i + di * step, j + dj * step
                    assert q in board
                moving = board[p][k:]
                assert len(board[q]) + len(moving) <= 8
                board[p] = board[p][:k]
                board[q] += moving[::-1]
                for cell in (p, q):
                    home = r['board'][cell[0]][cell[1]]
                    if home.isupper():
                        board[cell] = board[cell].rstrip(home.lower())
                b += 1
            assert q == tuple(r['operations'][next_a - 1]['destination'])
            state = tuple(board[p] for p in r['floor'])
            target = r['states'][next_a]
            assert all(len(x) == len(y) for x, y in zip(state, target))
            assert sum(x != y for x, y in zip(state, target)) <= 2
            assert b - (next_a - patch['begin']) <= 1
            peak = max(peak, b - (next_a - patch['begin']))
            a = next_a
        assert a == patch['end'] and b == len(patch['after'])
        assert tuple(board[p] for p in r['floor']) == r['states'][a]
        assert patch['saved'] == a - patch['begin'] - b > 0
        assert peak == patch['peak_excess'] and bool(relaxed) == bool(patch['relaxed'])
        assert patch['witness'] == (relaxed and patch['strict_saved'] < patch['saved'])
    return patches


def diagnostic():
    audit = verify_sources()
    rows = list(csv.DictReader((AUDIT / 'diagnostic.csv').open()))
    assert len(rows) == len({(r['policy'], r['group'], r['case'], r['record']) for r in rows}) == 990
    for row in rows:
        for key in row:
            if key not in ('policy', 'group', 'case', 'record'):
                row[key] = float(row[key]) if key == 'ms' else int(row[key])
        output = AUDIT / ('reduced_' + row['policy']) / row['group'] / row['case'] / (row['record'] + '.txt')
        assert verify_output(row['case'] + '.txt', output) == row['after'] <= row['before']
        assert row['before'] == len(plan_path(row).read_text().splitlines())
        assert row['saved'] == row['before'] - row['after'] and row['deadlines'] == 0
    patches = verify_patches()
    mechanism = json.loads((AUDIT / 'mechanism.json').read_text())
    assert mechanism['verified_outputs'] == 990 and mechanism['rng_consumption'] == 0
    assert mechanism['witnesses'] == sum(p['witness'] for p in patches)
    assert mechanism['overrun_witnesses'] == sum(p['witness'] and p['peak_excess'] > 0 for p in patches)
    result = {'solver_sha256': audit['solver_sha256'], 'mechanism': mechanism, 'groups': {}}
    for group in ('parent', 'initial', 'before'):
        selected = [r for r in rows if r['group'] == group]
        info = {}
        for policy in ('strict', 'slack'):
            rs = [r for r in selected if r['policy'] == policy]
            info[policy] = {'plans': len(rs), 'saved': sum(r['saved'] for r in rs),
                'shortened': sum(r['saved'] > 0 for r in rs),
                'mean_ms': sum(r['ms'] for r in rs) / len(rs), 'max_ms': max(r['ms'] for r in rs),
                'counts': {k: sum(r[k] for r in rs) for k in ('seeds', 'routes', 'branches', 'replacements',
                    'beam_caps', 'depth_caps', 'deadlines', 'overrun_seeds', 'relaxed_branches', 'relaxed_saved', 'overrun_saved')}}
        info['extra_saved'] = info['slack']['saved'] - info['strict']['saved']
        result['groups'][group] = info
    identical = 0
    for r in rows:
        if r['policy'] != 'slack':
            continue
        suffix = Path(r['group']) / r['case'] / (r['record'] + '.txt')
        identical += (AUDIT / 'reduced_slack' / suffix).read_bytes() == (AUDIT / 'reduced_strict' / suffix).read_bytes()
    result['identical_output_pairs'] = identical
    result['verified_patches'] = len(patches)
    result['proceed_to_evaluation'] = mechanism['witnesses'] > 0
    (AUDIT / 'diagnostic_verification.json').write_text(json.dumps({**result, 'cases': rows}, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    {'diagnostic': diagnostic}[sys.argv[1]]()
