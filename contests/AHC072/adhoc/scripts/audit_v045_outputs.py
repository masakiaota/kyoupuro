#!/usr/bin/env python3
"""既存出力を個体IDで再生する。解や操作列は生成しない。"""

import argparse
from collections import Counter, deque
import json
from pathlib import Path
import re

from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
DIRS = {'U': (-1, 0), 'D': (1, 0), 'L': (0, -1), 'R': (0, 1)}


def audit(input_path, output_path):
    base = replay(input_path, output_path)
    lines = input_path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = lines[1:N + 1]
    cells = [(i, j) for i in range(N) for j in range(N) if C[i][j] != '#']
    floors = set(cells)
    nests = {ord(C[i][j]) - 65: (i, j) for i, j in cells if C[i][j].isupper()}
    nest_color = {p: c for c, p in nests.items()}
    initial = [(i, j) for i, j in cells if C[i][j].islower()]
    color = [ord(C[i][j]) - 97 for i, j in initial]
    initial_id = {p: id_ for id_, p in enumerate(initial)}
    colors = Counter(color)

    def neighbors(p):
        for di, dj in DIRS.values():
            q = p[0] + di, p[1] + dj
            if q in floors:
                yield q

    def distances(source):
        d = {source: 0}
        queue = deque([source])
        while queue:
            p = queue.popleft()
            for q in neighbors(p):
                if q not in d:
                    d[q] = d[p] + 1
                    queue.append(q)
        return d

    nest_dist = {c: distances(p) for c, p in nests.items()}
    root_color = min(nests, key=lambda c: (sum(nest_dist[c][p] for p in initial), -colors[c], nests[c]))
    root = nests[root_color]
    depth = nest_dist[root_color]
    order = sorted(cells, key=lambda p: (depth[p], p))
    parent = {root: None}
    supply = {root: 0}
    for p in order:
        if p == root:
            continue
        parent[p] = min((q for q in neighbors(p) if depth[q] + 1 == depth[p]),
                        key=lambda q: (-supply[q], q))
        supply[p] = supply[parent[p]] + (p in initial_id)
    core = set()
    for p in nests.values():
        while p is not None and p not in core:
            core.add(p)
            p = parent[p]
    gateway = {}
    for p in order:
        gateway[p] = p if p in core else gateway[parent[p]]
    domain = [None if p in core else gateway[p] for p in initial]
    remaining = Counter(g for g in domain if g is not None)
    peripheral_remaining = sum(remaining.values())
    tower = {p: [initial_id[p]] if p in initial_id else [] for p in cells}
    alive = set(range(len(initial)))
    crossed = {id_ for id_, g in enumerate(domain) if g is None}
    pad_use = Counter()
    stats = Counter(core_cells=len(core), core_slimes=sum(g is None for g in domain),
                    regions=len(remaining), gate_passed=0, core_support_jumps=0,
                    core_support_repeated_uses=0, peripheral_T=0, cleanup_T=0)
    last_region = None
    finished_regions = set()
    for turn, line in enumerate(output_path.read_text().splitlines(), 1):
        if not line.strip():
            continue
        i, j, k, d, l = line.split()
        i, j, k, l = map(int, (i, j, k, l))
        p = i, j
        di, dj = DIRS[d]
        path = [(i + di * step, j + dj * step) for step in range(l + 1)]
        q = path[-1]
        moving = tower[p][k:]
        assert moving, (turn, 'empty')
        assert all(parent[a] == b or parent[b] == a for a, b in zip(path, path[1:])), (turn, 'tree')
        cleanup = peripheral_remaining == 0
        if cleanup:
            assert all(domain[id_] is None for id_ in moving), (turn, 'cleanup membership')
            assert all(a in core for a in path), (turn, 'cleanup route')
            stats['cleanup_T'] += 1
        else:
            domains = {domain[id_] for id_ in moving}
            assert len(domains) == 1 and None not in domains, (turn, 'core resident moved')
            active = next(iter(domains))
            if active != last_region:
                if last_region is not None:
                    assert remaining[last_region] == 0, (turn, 'unfinished region')
                    finished_regions.add(last_region)
                    assert (-depth[last_region], last_region) <= (-depth[active], active)
                assert active not in finished_regions, (turn, 'region reopened')
                last_region = active
            for a in path:
                assert a in core or (gateway[a] == active and remaining[active] > 0), (turn, 'region route')
            if any(a not in core for a in path):
                assert l == 1, (turn, 'pickup jump')
            if l > 1 and p in core and p in initial_id:
                assert k == 1 and tower[p][0] == initial_id[p], (turn, 'support not preserved')
                pad_use[initial_id[p]] += 1
                stats['core_support_jumps'] += 1
            stats['peripheral_T'] += 1
        for id_ in moving:
            if q == domain[id_]:
                crossed.add(id_)
        tower[p] = tower[p][:k]
        tower[q].extend(reversed(moving))
        for here in (p, q):
            while tower[here] and color[tower[here][-1]] == nest_color.get(here):
                id_ = tower[here].pop()
                assert id_ in alive and id_ in crossed, (turn, 'unvisited gateway')
                alive.remove(id_)
                if domain[id_] is not None:
                    remaining[domain[id_]] -= 1
                    peripheral_remaining -= 1
                    stats['gate_passed'] += 1
                else:
                    assert cleanup, (turn, 'early core return')
    assert not alive and not any(remaining.values()), 'incomplete'
    stats['core_support_repeated_uses'] = sum(max(0, n - 1) for n in pad_use.values())
    stats['core_supports_used'] = len(pad_use)
    err = output_path.with_name(output_path.name + '.err')
    trace = {}
    if err.exists():
        trace = {m[1]: int(m[2]) for m in re.finditer(r'^\[summary.count\] (\w+)=(\d+)$', err.read_text(), re.M)}
        for key, actual in stats.items():
            assert trace[key] == actual, (key, trace[key], actual)
        assert trace['final_T'] == base['metrics']['T']
        assert trace['remaining_slimes'] == base['metrics']['E'] == 0
        assert trace['transport_work'] == base['metrics']['W']
    return {'case': input_path.name, 'metrics': base['metrics'], 'structure': dict(stats), 'trace': trace}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output_dir', type=Path)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    outputs = sorted(args.output_dir.glob('[0-9][0-9][0-9][0-9].txt'))
    assert outputs
    cases = [audit(ROOT / 'tools/in' / output.name, output) for output in outputs]
    normal = [case for case in cases if case['case'] != '0000.txt']
    totals = Counter()
    for case in normal:
        totals.update(case['structure'])
    coverage = {key: sum(case['structure'].get(key, 0) > 0 for case in normal)
                for key in ('core_support_jumps', 'core_support_repeated_uses')}
    coverage['peripheral_mixed_groups'] = sum(case['trace'].get('peripheral_mixed_groups', 0) > 0 for case in normal)
    report = {'cases': cases, 'normal_structure_totals': dict(totals), 'normal_coverage': coverage,
              'violations': 0}
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'cases': len(cases), 'violations': 0, 'normal_structure_totals': dict(totals),
                      'normal_coverage': coverage}, ensure_ascii=False))


if __name__ == '__main__':
    main()
