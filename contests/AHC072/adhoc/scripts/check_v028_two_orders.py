#!/usr/bin/env python3
"""Independently replay all frozen two-order diagnostic completions."""
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v028_audit"
DIRECTIONS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def validate(case, moves):
    rows = (ROOT / "tools/in" / case).read_text().splitlines()
    n, _ = map(int, rows[0].split())
    cells = "".join(rows[1:n+1])
    towers = [[ord(c)-ord("a")] if "a" <= c <= "z" else [] for c in cells]
    homes = {p: ord(c)-ord("A") for p, c in enumerate(cells) if "A" <= c <= "Z"}
    assert len(moves) <= 100000
    for row, col, keep, direction, length in moves:
        assert 0 <= row < n and 0 <= col < n and direction in DIRECTIONS
        p = row*n+col
        assert 0 <= keep < len(towers[p]) and 1 <= length <= keep+1
        dr, dc = DIRECTIONS[direction]
        for step in range(1, length+1):
            r, c = row+dr*step, col+dc*step
            assert 0 <= r < n and 0 <= c < n and cells[r*n+c] != "#"
        q = (row+dr*length)*n+col+dc*length
        moving = towers[p][keep:]
        assert len(towers[q])+len(moving) <= 8
        towers[p] = towers[p][:keep]
        towers[q].extend(reversed(moving))
        for at in (p, q):
            while towers[at] and towers[at][-1] == homes.get(at, -1):
                towers[at].pop()
    assert not any(towers), case


def main():
    with (OUT / "diagnostic.csv").open() as f:
        rows = list(csv.DictReader(f))
    by_key = {(r["case"], r["hash"]): r for r in rows}
    assert len(rows) == len(by_key)
    total, per_case = Counter(), {}
    for line in (OUT / "diagnostic_moves.jsonl").open():
        record = json.loads(line)
        row = by_key.pop((record["case"], record["hash"]))
        count = per_case.setdefault(record["case"], Counter())
        count["candidates"] += 1
        count["extracted"] += int(row["extracted"])
        count["dependency_candidates"] += int(row["dependency"])
        count[f"size_{row['size']}"] += 1
        count["alternate_selected"] += int(row["alternate_selected"])
        count["rescued"] += int(row["rescued"])
        count["raw_saved"] += int(row["raw_saved"])
        for kind in ("primary", "alternate", "chosen"):
            moves = record[kind]
            assert bool(int(row[kind+"_ok"])) == (moves is not None)
            assert int(row[kind+"_T"]) == (len(moves) if moves is not None else -1)
            if moves is not None:
                validate(record["case"], moves)
                count[kind+"_valid"] += 1
        primary, alternate, chosen = (record[k] for k in ("primary", "alternate", "chosen"))
        expected = alternate if alternate is not None and (primary is None or len(alternate) < len(primary)) else primary
        assert chosen == expected
    assert not by_key and len(per_case) == 100
    for count in per_case.values():
        assert 0 < count["candidates"] <= 64
        total.update(count)
    assert total["alternate_selected"] > 0
    audit = json.loads((OUT / "static_verification.json").read_text())
    source = ROOT / "src/bin/v028_two_order_lns.cpp"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == audit["solver_sha256"]
    result = {"verified_cases": 100, "total": dict(total),
              "alternate_selected_cases": sum(c["alternate_selected"] > 0 for c in per_case.values()),
              "solver_sha256": audit["solver_sha256"], "cases": per_case}
    (OUT / "diagnostic_check.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps({k: v for k, v in result.items() if k != "cases"}, indent=2))


if __name__ == "__main__":
    main()
