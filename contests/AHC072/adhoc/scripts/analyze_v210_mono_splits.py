#!/usr/bin/env python3
"""保存済みv210の同色分割を数える。solverの起動や変更は行わない。"""
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "results/analysis/v210_identity_review"
DIRECTIONS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def main():
    totals, details, examples, hashes = Counter(), [], [], {}
    for input_path in sorted((ROOT / "tools/in").glob("*.txt")):
        raw = input_path.read_text().splitlines()
        N, _ = map(int, raw[0].split())
        grid = "".join(raw[1:N+1])
        assert len(grid) == N*N
        towers = [[ord(c)-ord("a")] if "a" <= c <= "l" else [] for c in grid]
        nests = {p: ord(c)-ord("A") for p, c in enumerate(grid) if "A" <= c <= "L"}
        answer_path = ROOT / "results/out/v210_collection_color_quotient" / input_path.name
        for path in (input_path, answer_path):
            hashes[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
        count = Counter()
        for time, line in enumerate(answer_path.read_text().splitlines()):
            row, col, keep, direction, length = line.split()
            row, col, keep, length = map(int, (row, col, keep, length))
            p = row*N+col
            before = towers[p]
            assert 0 <= keep < len(before) and 1 <= length <= keep+1
            # 実際の出発マスに限り、合法な切断位置・方向・距離の組数を比較する。
            for k in range(len(before)):
                cuts_same = k > 0 and before[k-1] == before[k]
                for dr, dc in DIRECTIONS.values():
                    for distance in range(1, k+2):
                        r, c = row+dr*distance, col+dc*distance
                        if not (0 <= r < N and 0 <= c < N) or grid[r*N+c] == "#":
                            break
                        if len(towers[r*N+c])+len(before)-k <= 8:
                            count["legal_moves_at_chosen_source"] += 1
                            count["forbidden_moves_at_chosen_source"] += cuts_same
            dr, dc = DIRECTIONS[direction]
            for distance in range(1, length+1):
                r, c = row+dr*distance, col+dc*distance
                assert 0 <= r < N and 0 <= c < N and grid[r*N+c] != "#"
            q = (row+dr*length)*N+col+dc*length
            assert len(towers[q])+len(before)-keep <= 8
            same_split = keep > 0 and before[keep-1] == before[keep]
            count["operations"] += 1
            count["partial_moves"] += keep > 0
            count["same_color_splits"] += same_split
            count["same_color_split_long_jumps"] += same_split and length > 1
            count["same_color_split_mono_tower"] += same_split and len(set(before)) == 1
            if same_split and length > 1 and len(examples) < 12:
                examples.append(dict(case=input_path.name, operation=time+1, move=line,
                    departure_colors=before.copy(), destination_colors=towers[q].copy(),
                    destination_nest=nests.get(q)))
            moving = before[keep:]
            towers[p] = before[:keep]
            towers[q].extend(reversed(moving))
            for cell in (p, q):
                while towers[cell] and towers[cell][-1] == nests.get(cell, -1):
                    towers[cell].pop()
        assert not any(towers), input_path.name
        totals.update(count)
        details.append(dict(case=input_path.name, **count))
    assert len(details) == 100 and totals["operations"] == 20130
    result = dict(cases=100, counts=dict(totals),
        split_cases=sum(row["same_color_splits"] > 0 for row in details),
        split_operation_fraction=totals["same_color_splits"]/totals["operations"],
        forbidden_fraction_at_chosen_sources=totals["forbidden_moves_at_chosen_source"]/totals["legal_moves_at_chosen_source"],
        examples=examples, details=details, input_output_sha256=hashes, solver_executions=0)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "mono_splits.json").write_text(json.dumps(result, ensure_ascii=False, indent=2)+"\n")
    print(json.dumps({key: value for key, value in result.items()
                     if key not in ("examples", "details", "input_output_sha256")}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
