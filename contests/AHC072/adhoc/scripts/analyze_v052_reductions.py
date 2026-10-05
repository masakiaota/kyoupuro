#!/usr/bin/env python3
"""Independently replay saved replacements and explain their local boundaries."""
import csv
import json
from pathlib import Path
import re

from analyze_v047_coordination import inspect

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v052_audit'


def operation(op):
    colors = lambda values: ''.join(v[0] for v in values)
    return {key: op[key] for key in ('turn', 'action', 'source', 'destination', 'k', 'length')} | {
        key: colors(op[key]) for key in ('moving', 'support', 'landing', 'returned_source',
                                        'returned_destination', 'before_source', 'before_destination',
                                        'after_source', 'after_destination')}


def main():
    candidates = {}
    for row in csv.DictReader((AUDIT / 'candidates.csv').open()):
        key = row['case'], int(row['first']), int(row['last'])
        candidates[key] = {
            'cells': [tuple(map(int, cell.split(':'))) for cell in row['cells'].strip(';').split(';')],
            'operations': list(map(int, row['operations'].strip(';').split(';'))),
            'has_nest': bool(int(row['has_nest'])),
        }
    patches = {}
    lines = (AUDIT / 'patches.txt').read_text().splitlines()
    at = 0
    while at < len(lines):
        found = re.fullmatch(r'(\d{4}\.txt) \[(\d+),(\d+)\] (\d+) -> (\d+) cells=(\d+) pieces=(\d+)', lines[at])
        assert found, lines[at]
        case = found[1]
        first, last, before_count, after_count, cell_count, pieces = map(int, found.groups()[1:])
        moves = lines[at + 1:at + 1 + after_count]
        assert len(moves) == after_count
        patches.setdefault(case, []).append(dict(first=first, last=last, before_count=before_count,
                                                after_count=after_count, cell_count=cell_count,
                                                pieces=pieces, moves=moves))
        at += 1 + after_count

    results = []
    for case, replacements in patches.items():
        parent_path = ROOT / 'results/out/v050_joint_towers' / case
        child_path = AUDIT / 'compressed_v050' / case
        inp = ROOT / 'tools/in' / case
        parent, child = inspect(inp, parent_path), inspect(inp, child_path)
        original = parent_path.read_text().splitlines()
        floor = parent['floor']
        positions = {p: i for i, p in enumerate(floor)}
        assert floor == child['floor']
        expected, cursor, shift = [], 0, 0
        for patch in replacements:
            first, last = patch['first'], patch['last']
            window = candidates[case, first, last]
            cells, related = set(window['cells']), set(window['operations'])
            assert cursor <= first and len(related) == patch['before_count']
            assert len(cells) == patch['cell_count']
            retained = [t for t in range(first, last + 1) if t not in related]
            for t in retained:
                op = parent['operations'][t]
                assert tuple(op['source']) not in cells and tuple(op['destination']) not in cells
            new_first = first + shift
            new_end = new_first + patch['after_count'] + len(retained)
            assert parent['states'][first] == child['states'][new_first]
            assert parent['states'][last + 1] == child['states'][new_end]
            # The new route runs before unrelated old operations. At this
            # boundary every background tower must still equal the entrance.
            target = list(parent['states'][first])
            for cell in cells:
                target[positions[cell]] = parent['states'][last + 1][positions[cell]]
            assert tuple(target) == child['states'][new_first + patch['after_count']]
            expected.extend(original[cursor:first])
            expected.extend(patch['moves'])
            expected.extend(original[t] for t in retained)
            cursor = last + 1
            shift += patch['after_count'] - patch['before_count']
            before = [operation(parent['operations'][t]) for t in sorted(related)]
            after = [operation(op) for op in child['operations'][new_first:new_first + patch['after_count']]]
            assert [op['action'] for op in after] == patch['moves']
            touched = sorted(cells | {tuple(op[key]) for op in after for key in ('source', 'destination')})
            initial = {str(p): parent['states'][first][positions[p]] for p in touched}
            final = {str(p): target[positions[p]] for p in touched}
            results.append(dict(case=case, **patch, saved=patch['before_count'] - patch['after_count'],
                                has_nest=window['has_nest'], independent_operations=len(retained),
                                full_board_boundaries_equal=True, background_restored_before_retained=True,
                                cells=window['cells'], initial=initial, final=final, before=before, after=after))
        expected.extend(original[cursor:])
        assert expected == child_path.read_text().splitlines()
    result = dict(patches=len(results), cases=len(patches), saved=sum(r['saved'] for r in results),
                  independent_operations=sum(r['independent_operations'] for r in results),
                  all_full_board_boundaries_equal=True, all_backgrounds_restored=True, reductions=results)
    assert result['patches'] == 20 and result['saved'] == 24
    (AUDIT / 'reduction_examples.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'reductions'}, indent=2))


if __name__ == '__main__':
    main()
