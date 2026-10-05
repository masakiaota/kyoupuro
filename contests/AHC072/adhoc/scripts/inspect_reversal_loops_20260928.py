#!/usr/bin/env python3
"""Describe reversal loops in saved outputs; never generate or modify moves."""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[2]
DIRECTIONS = {'U': (-1, 0), 'D': (1, 0), 'L': (0, -1), 'R': (0, 1)}


def inspect(case, output):
    lines = case.read_text().splitlines()
    N, K = map(int, lines[0].split())
    cells = ''.join(lines[1:N + 1])
    towers = [[p] if ch.islower() else [] for p, ch in enumerate(cells)]
    previous = None
    chain = []
    found = []
    for turn, line in enumerate(output.read_text().splitlines(), 1):
        i, j, k, direction, length = line.split()
        i, j, k, length = map(int, (i, j, k, length))
        p = i*N + j
        di, dj = DIRECTIONS[direction]
        q = (i + di*length)*N + j + dj*length
        assert 0 <= k < len(towers[p]) and 1 <= length <= k + 1
        for offset in range(1, length + 1):
            ni, nj = i + di*offset, j + dj*offset
            assert 0 <= ni < N and 0 <= nj < N and cells[ni*N + nj] != '#'
        moving = towers[p][k:]
        assert len(towers[q]) + len(moving) <= 8
        ids = frozenset(moving)
        if previous != (ids, p):
            chain = []
        chain.append((p, q, turn, [cells[x] for x in moving]))
        if len(chain) >= 3:
            recent = chain[-3:]
            word = recent[0][3]
            # Three moves reverse a packet. A palindrome does not change its
            # visible order, so it is outside this particular observation.
            if recent[0][0] == q and word != word[::-1]:
                found.append({'case': case.name, 'first_turn': recent[0][2],
                              'last_turn': turn, 'size': len(ids),
                              'word': ''.join(word),
                              'path': [list(divmod(recent[0][0], N))]
                                      + [list(divmod(move[1], N)) for move in recent]})
        towers[p] = towers[p][:k]
        towers[q].extend(reversed(moving))
        for at in (p, q):
            while towers[at] and cells[towers[at][-1]].upper() == cells[at]:
                towers[at].pop()
        previous = (ids, q)
    assert not any(towers)
    return found


if __name__ == '__main__':
    cases = sorted((ROOT/'tools/in').glob('*.txt'))
    loops = []
    for case in cases:
        loops.extend(inspect(case, ROOT/'results/out/v039_exact_board_lns'/case.name))
    summary = {'cases': len(cases), 'loops': len(loops),
               'cases_with_loops': len({item['case'] for item in loops}),
               'note': 'Consecutive three-move returns of the same mixed packet. Counts are not guaranteed reductions.',
               'details': loops}
    destination = ROOT/'adhoc/play_saved_edits_20260928/reversal_loops.json'
    destination.write_text(json.dumps(summary, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({key: value for key, value in summary.items() if key != 'details'}, ensure_ascii=False))
    print(json.dumps([item for item in loops if item['case'] == '0060.txt'], ensure_ascii=False))
