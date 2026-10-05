#!/usr/bin/env python3
"""Inventory finite connected windows in saved answers; never generate moves."""
from collections import Counter
import json
from pathlib import Path

from analyze_v047_coordination import inspect

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/finite_window_inventory_20260929'
LONG = ROOT / 'results/long_search/v047/20260929T005348_ca87d7da'


def windows(report):
    positions = {p: i for i, p in enumerate(report['floor'])}
    operations = report['operations']
    for first in range(len(operations)):
        selected, blocked, related = {}, set(), []
        pieces = towers = 0
        for at in range(first, min(first + 64, len(operations))):
            op = operations[at]
            p, q = tuple(op['source']), tuple(op['destination'])
            if at != first and p not in selected and q not in selected:
                blocked.update((p, q))
                continue
            added = [v for v in (p, q) if v not in selected]
            if any(v in blocked for v in added) or len(selected) + len(added) > 8:
                break
            words = [report['states'][first][positions[v]] for v in added]
            if pieces + sum(map(len, words)) > 16 or towers + sum(bool(w) for w in words) > 6:
                break
            pieces += sum(map(len, words))
            towers += sum(bool(w) for w in words)
            for v, word in zip(added, words):
                selected[v] = word
            related.append(at)
            if len(related) >= 3 and pieces >= 2:
                final = {v: report['states'][at + 1][positions[v]] for v in selected}
                yield {'first': first, 'last': at, 'operations': related.copy(), 'pieces': pieces,
                       'cells': sorted(selected), 'towers': towers,
                       'changed': sum(final[v] != selected[v] for v in selected),
                       'remaining': sum(map(len, final.values())),
                       'has_nest': any(report['board'][v[0]][v[1]].isupper() for v in selected)}
            if len(related) == 12:
                break


def main():
    OUT.mkdir(exist_ok=True)
    cases = []
    total = Counter()
    for inp in sorted((ROOT / 'tools/in').glob('*.txt')):
        report = inspect(inp, ROOT / 'results/out/v050_joint_towers' / inp.name)
        found = list(windows(report))
        counts = Counter(candidates=len(found), starts=len({w['first'] for w in found}),
                         nonempty_goals=sum(w['remaining'] > 0 for w in found),
                         more_than_four_cells=sum(len(w['cells']) > 4 for w in found),
                         nest_windows=sum(w['has_nest'] for w in found),
                         middle_starts=len({w['first'] for w in found if len(report['operations']) // 4 <= w['first'] < 3 * len(report['operations']) // 4}))
        total.update(counts)
        cases.append({'case': inp.name, 'T': len(report['operations']), **counts})
    witnesses = []
    for name in ('consolidated_dispatch', 'nest_pickup', 'pass_before_merge', 'paired_orientation'):
        reference = json.loads((LONG / 'analysis/coordination' / (name + '.json')).read_text())
        inp = ROOT / 'tools/in' / (reference['case'] + '.txt')
        report = inspect(inp, LONG / reference['before_plan'])
        match = [w for w in windows(report) if w['first'] == reference['before_start'] and w['last'] + 1 == reference['before_end']]
        witnesses.append({'name': name, 'case': reference['case'], 'found': bool(match),
                          'before': reference['before_moves'], 'after': reference['after_moves'],
                          'window': match[0] if match else None})
    result = {'limits': {'cells': 8, 'pieces': 16, 'initial_towers': 6, 'related_operations': 12, 'scan_operations': 64},
              'cases': cases, 'total': total, 'eligible_cases': sum(r['candidates'] > 0 for r in cases),
              'middle_eligible_cases': sum(r['middle_starts'] > 0 for r in cases), 'witnesses': witnesses}
    (OUT / 'inventory.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'cases'}, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
