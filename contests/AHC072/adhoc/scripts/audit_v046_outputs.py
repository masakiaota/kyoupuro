#!/usr/bin/env python3
"""保存した構築解と最終解を個体IDで再生する。操作は生成しない。"""

import argparse
from collections import Counter
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
DIRECTIONS = {'U': (-1, 0), 'D': (1, 0), 'L': (0, -1), 'R': (0, 1)}


def geometry(path):
    lines = path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = lines[1:N + 1]
    cells = [(i, j) for i in range(N) for j in range(N) if C[i][j] != '#']
    index = {p: i for i, p in enumerate(cells)}
    nests = {index[i, j]: ord(C[i][j]) - 64 for i, j in cells if C[i][j].isupper()}
    initial = {index[i, j]: ord(C[i][j]) - 96 for i, j in cells if C[i][j].islower()}
    return N, K, cells, index, nests, initial


def validate_routes(g, routes):
    _, K, cells, index, nests, _ = g
    for color in range(1, K + 1):
        assert color in routes and len(routes[color]) == len(cells)
        home = next(p for p, c in nests.items() if c == color)
        assert routes[color][home] == 'X'
        for source in range(len(cells)):
            visited = set()
            p = source
            while p != home:
                assert p not in visited, ('route cycle', color, source)
                visited.add(p)
                d = routes[color][p]
                assert d in DIRECTIONS, ('route endpoint', color, p)
                di, dj = DIRECTIONS[d]
                i, j = cells[p]
                assert (i + di, j + dj) in index
                p = index[i + di, j + dj]


def replay(g, lines, routes=None, events=None):
    _, _, cells, index, nests, initial = g
    tower = {p: [p] if p in initial else [] for p in range(len(cells))}
    alive = set(initial)
    met = Counter(T=0, E=len(initial), W=0, mixed_moves=0, long_jumps=0,
                  immediate_reversals=0, reversals_after_pickup=0, max_height=1,
                  source_returned=0, destination_returned=0, shared_route_mixed_moves=0,
                  shared_route_mixed_distance=0, shared_route_long_jumps=0,
                  new_joint_carry_after_home=0, pending_support_long_jumps=0,
                  guided_merge_moves=0, guided_forward_moves=0)
    history = {p: 1 << p for p in initial}
    after_home = {}
    previous = None
    witnesses = {}
    actions = [line.strip() for line in lines if line.strip()]
    event_kind = [None] * len(actions)
    if events is not None:
        cursor = 0
        for begin, end, kind, *_ in events:
            assert begin == cursor and begin <= end <= len(actions), ('event coverage', begin, end, cursor)
            event_kind[begin:end] = [kind] * (end - begin)
            cursor = end
        assert cursor == len(actions)
    for turn, line in enumerate(actions):
        i, j, k, d, l = line.split()
        i, j, k, l = map(int, (i, j, k, l))
        assert (i, j) in index and d in DIRECTIONS
        p = index[i, j]
        assert 0 <= k < len(tower[p]) and 1 <= l <= min(8, k + 1), (turn, 'departure')
        di, dj = DIRECTIONS[d]
        path = []
        for step in range(l + 1):
            point = (i + step * di, j + step * dj)
            assert point in index, (turn, 'wall')
            path.append(index[point])
        q = path[-1]
        moving = tower[p][k:]
        source_before = tower[p][:]
        resident = tower[q][:]
        assert len(resident) + len(moving) <= 8, (turn, 'capacity')
        met['max_height'] = max(met['max_height'], len(resident) + len(moving))
        colors = {initial[id_] for id_ in moving}
        moving_bits = sum(1 << id_ for id_ in moving)
        if any(id_ in after_home and moving_bits & ~after_home[id_] for id_ in moving):
            met['new_joint_carry_after_home'] += 1
            witnesses.setdefault('new_joint_carry_after_home', turn + 1)
        followed = routes is not None and all(routes[c][a] == d for c in colors for a in path[:-1])
        if followed and len(colors) >= 2:
            met['shared_route_mixed_moves'] += 1
            met['shared_route_mixed_distance'] += l
            witnesses.setdefault('shared_route_mixed_moves', turn + 1)
        if followed and l > 1:
            met['shared_route_long_jumps'] += 1
        kind = event_kind[turn]
        if kind in (0, 1):
            assert followed, (turn, 'guided move leaves network')
            met['guided_merge_moves' if kind == 0 else 'guided_forward_moves'] += 1
        if previous is not None:
            old_p, old_q, old_ids = previous
            if old_p == q and old_q == p and set(moving) & old_ids:
                met['immediate_reversals'] += 1
                met['reversals_after_pickup'] += bool(set(moving) - old_ids)
                witnesses.setdefault('immediate_reversal', turn)
        previous = (p, q, set(moving))
        tower[p] = tower[p][:k]
        tower[q].extend(reversed(moving))
        together = sum(1 << id_ for id_ in tower[q])
        for id_ in tower[q]:
            history[id_] |= together
        returned = []
        for here, key in ((p, 'source_returned'), (q, 'destination_returned')):
            while tower[here] and initial[tower[here][-1]] == nests.get(here):
                id_ = tower[here].pop()
                assert id_ in alive
                alive.remove(id_)
                returned.append(id_)
                met[key] += 1
        if returned:
            for id_ in set(source_before + resident) & alive:
                after_home.setdefault(id_, history[id_])
        if l > 1 and tower[p]:
            met['pending_support_long_jumps'] += 1
            witnesses.setdefault('pending_support_long_jump', turn + 1)
        met['T'] += 1
        met['W'] += len(moving) * l
        met['mixed_moves'] += len(colors) >= 2
        met['long_jumps'] += l > 1
    met['E'] = len(alive)
    assert not alive, ('not delivered', len(alive))
    assert met['T'] <= 100000
    return {'metrics': dict(met), 'witnesses': witnesses}


def audit_case(output):
    g = geometry(ROOT / 'tools/in' / output.name)
    text = output.with_name(output.name + '.err').read_text()
    trace = {m[1]: int(m[2]) for m in re.finditer(r'^\[summary.count\] (\w+)=(-?\d+)$', text, re.M)}
    for key in ('construction_errors', 'lns_errors', 'baseline_recovery', 'final_recovery', 'lns_invalid_candidates'):
        assert trace.get(key, 0) == 0, (output.name, key, trace.get(key))
    routes = {int(m[1]): m[2] for m in re.finditer(r'^\[network.route\] (\d+) ([UDLRX]+)$', text, re.M)}
    if routes:
        validate_routes(g, routes)
    result = {'case': output.name, 'trace': trace, 'final': replay(g, output.read_text().splitlines(), routes or None)}
    assert result['final']['metrics']['T'] == trace['T'] and trace['E'] == 0
    seed_lines = re.findall(r'^\[network.seed\] (.+)$', text, re.M)
    if seed_lines:
        events = [tuple(map(int, m.split())) for m in re.findall(r'^\[network.event\] (.+)$', text, re.M)]
        result['seed'] = replay(g, seed_lines, routes, events)
        main = re.search(r'\bpre_lns=(\d+)', text)
        assert main and result['seed']['metrics']['T'] == int(main[1])
        assert sum(e[2] == 0 for e in events) == trace['network_seed_merges']
        assert sum(e[2] == 1 for e in events) == trace['network_seed_forward_events']
        assert sum(e[2] == 2 for e in events) == trace['network_seed_delivery_events']
        result['events'] = events
    else:
        assert trace['network_seed_selected'] == 0
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output_dir', type=Path)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    paths = sorted(args.output_dir.glob('[0-9][0-9][0-9][0-9].txt'))
    assert paths
    cases = [audit_case(p) for p in paths]
    normal = [c for c in cases if c['case'] != '0000.txt']
    totals = {'final': Counter(), 'seed': Counter(), 'trace': Counter()}
    for case in normal:
        totals['final'].update(case['final']['metrics'])
        totals['trace'].update(case['trace'])
        if 'seed' in case:
            totals['seed'].update(case['seed']['metrics'])
    fields = ['mixed_moves', 'long_jumps', 'new_joint_carry_after_home', 'shared_route_mixed_moves',
              'shared_route_long_jumps', 'pending_support_long_jumps', 'immediate_reversals']
    coverage = {stage: {key: sum(case.get(stage, {}).get('metrics', {}).get(key, 0) > 0 for case in normal)
                        for key in fields} for stage in ['seed', 'final']}
    report = {'cases': cases, 'normal_totals': {k: dict(v) for k, v in totals.items()},
              'normal_coverage': coverage, 'violations': 0}
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'cases': len(cases), 'violations': 0,
                      'selected_network_seeds': sum('seed' in c for c in cases),
                      'normal_final_T': totals['final']['T'], 'normal_coverage': coverage}, ensure_ascii=False))


if __name__ == '__main__':
    main()
