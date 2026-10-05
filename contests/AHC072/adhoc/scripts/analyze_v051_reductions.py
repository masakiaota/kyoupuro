#!/usr/bin/env python3
"""Explain saved v051 reductions using equal full-board boundaries; no search."""
from collections import Counter
import json
from pathlib import Path

from analyze_v047_coordination import inspect, closed_segments
from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v051_audit'


def main():
    diagnostic = json.loads((AUDIT / 'diagnostic_verification.json').read_text())
    examples = []
    for row in diagnostic['cases']:
        if row['mode'] != 'combined' or row['saved'] == 0:
            continue
        case = row['case']
        inp = ROOT / 'tools/in' / case
        original = ROOT / 'results/out/v050_joint_towers' / case
        reduced = AUDIT / 'combined' / case
        a, b = inspect(inp, original), inspect(inp, reduced)
        ra, rb = replay(inp, original), replay(inp, reduced)
        segments = closed_segments(a, b)
        assert sum(s['saved'] for s in segments) == row['saved']
        for segment in segments:
            a0, a1 = segment['before_start'], segment['before_end']
            b0, b1 = segment['after_start'], segment['after_end']
            assert a['states'][a0] == b['states'][b0] and a['states'][a1] == b['states'][b1]
            before = ra['operations'][a0:a1]
            after = rb['operations'][b0:b1]
            def signature(op):
                return (op['action'], tuple(sorted(op['before'].items())), tuple(sorted(op['after'].items())))

            common = Counter(signature(op) for op in before) & Counter(signature(op) for op in after)

            def changed(operations):
                remaining = common.copy()
                result = []
                for op in operations:
                    if remaining[signature(op)]:
                        remaining[signature(op)] -= 1
                    else:
                        result.append(op)
                return result

            old_changes, new_changes = changed(before), changed(after)
            touched = {tuple(op[key]) for op in old_changes + new_changes for key in ('source', 'destination')}
            initial = {str(p): a['states'][a0][i] for i, p in enumerate(a['floor']) if p in touched}
            final = {str(p): a['states'][a1][i] for i, p in enumerate(a['floor']) if p in touched}
            # Equal local transitions alone do not prove commutation. Verify that
            # the retained operations actually avoid all changed endpoints.
            independent = all(not any(tuple(op[key]) in touched for key in ('source', 'destination'))
                              for op in before if op not in old_changes)
            examples.append({'case': case, **segment, 'same_full_boundary_boards': True,
                             'initial_changed_cells': initial, 'final_changed_cells': final,
                             'retained_operations': sum(common.values()), 'retained_independent': independent,
                             'before_changed_operations': old_changes, 'after_changed_operations': new_changes})
    result = {'source': 'frozen v050 outputs and v051 diagnostic outputs',
              'total_saved': sum(x['saved'] for x in examples), 'examples': examples}
    (AUDIT / 'reduction_examples.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    for example in examples:
        print(example['case'], f"{example['before_moves']} -> {example['after_moves']}",
              'initial', example['initial_changed_cells'], 'retained_independent', example['retained_independent'])
        for name in ('before', 'after'):
            for op in example[f'{name}_changed_operations']:
                print(name, op['action'], op['before'], 'move', op['moving_bottom_to_top'], 'end', op['after'])


if __name__ == '__main__':
    main()
