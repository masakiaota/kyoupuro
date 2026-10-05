#!/usr/bin/env python3
"""Apply the v071 cleanup directly to v058, retaining its search strategy."""
from pathlib import Path
import difflib
import json

import refactor_v071 as common
from refactor_v071 import rename

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT / 'src/bin/v058_joint_priority.cpp'
CHILD = ROOT / 'src/bin/v072_joint_refactored.cpp'
OUT = ROOT / 'adhoc/v072_audit'


def prepare():
    source = PARENT.read_text()
    reference = common.PARENT.read_text().splitlines(True)
    comments = {}

    def bind(old, new):
        assert source.count(old) == 1, old
        start = source[:source.index(old)].count('\n') + 1
        assert start not in comments, start
        comments[start] = new

    # Match identical comment blocks by content: line numbers diverge after
    # v058's schedule. Do not transfer v059's repair-priority explanations.
    for start, content in common.COMMENTS.items():
        if start in (1, 2331, 3285):
            continue
        end = start
        while end < len(reference) and reference[end].lstrip().startswith('//'):
            end += 1
        bind(''.join(reference[start - 1:end]), content)

    lines = source.splitlines(True)
    end = next(i for i, line in enumerate(lines) if not line.startswith('//'))
    bind(''.join(lines[:end]), 'v072_joint_refactored.cpp\n探索の実数演算の順序を保つためfast-mathは使わない。')
    bind('    // v058: reserve 3/4 of calls for joint shortening within the same time budget.\n'
         '    // Keep each other method once per cycle; advancing still requires budget.\n',
         '同じ累積予算で共同短縮を9/12回呼び、他の3処理を各1回挟む。予算があるときだけ進む。')
    # The common cleanup places the dependency comment at the function itself.
    bind('    // Grow only when removing the seed actually breaks a surviving flight.\n'
         "    // Completing the flight's removal releases its route for the existing DP.\n", '')
    bind('                    // Several reducers may contribute to the same new-best candidate.\n'
         '                    // These per-kind counts record involvement, not isolated causality.\n',
         '複数の短縮が同じ最良更新に関わり得る。種類別の関与を数え、単独効果とは扱わない。')

    previous = common.PARENT, common.COMMENTS
    try:
        common.PARENT, common.COMMENTS = PARENT, dict(sorted(comments.items()))
        child, changes, mapping = common.prepare()
    finally:
        common.PARENT, common.COMMENTS = previous

    schedule = ('    static constexpr array<int,12> heavy_kinds={\n'
                '        FINAL_JOINT,FINAL_JOINT,FINAL_JOINT,FINAL_STRICT,\n'
                '        FINAL_JOINT,FINAL_JOINT,FINAL_JOINT,FINAL_SLACK,\n'
                '        FINAL_JOINT,FINAL_JOINT,FINAL_JOINT,FINAL_FINITE,\n'
                '    };')
    assert schedule in child
    assert 'cand.priority=(saved+0.25)/divisor[cand.ids.size()]*(0.90+0.20*rng.unit());' in child
    assert 'broken_support_jumps' not in child and 'repair_priority' not in child
    assert child.count('best_created_with') == source.count('best_created_with') == 4
    return child, changes, mapping


if __name__ == '__main__':
    assert not (OUT / 'frozen.json').exists(), 'Already executed: source is frozen.'
    OUT.mkdir(exist_ok=True)
    source, changes, mapping = prepare()
    CHILD.write_text(source)
    (OUT / 'registered_changes.json').write_text(json.dumps(
        {'edits': changes, 'renames': mapping}, ensure_ascii=False, indent=2) + '\n')
    (OUT / 'source.diff').write_text(''.join(difflib.unified_diff(
        PARENT.read_text().splitlines(True), source.splitlines(True),
        fromfile=PARENT.name, tofile=CHILD.name)))
    print({'before_lines': len(PARENT.read_text().splitlines()),
           'after_lines': len(source.splitlines()), 'renames': len(mapping),
           'registered_edits': len(changes)})
