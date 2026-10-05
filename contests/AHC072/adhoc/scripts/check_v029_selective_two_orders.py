#!/usr/bin/env python3
"""Check the gate against frozen v028 routes, then independently replay outputs."""
from collections import Counter
import csv
import hashlib
from itertools import zip_longest
import json
from pathlib import Path

from check_v028_two_orders import validate

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v029_audit"
PRIOR = ROOT / "adhoc/v028_audit"


def main():
    audit = json.loads((OUT / "static_verification.json").read_text())
    old_analysis = json.loads((ROOT / "adhoc/v028_behavior_analysis/summary.json").read_text())
    for name in ("diagnostic.csv", "diagnostic_moves.jsonl"):
        path = PRIOR / name
        assert hashlib.sha256(path.read_bytes()).hexdigest() == old_analysis["input_hashes"][str(path.relative_to(ROOT))]
    rows = list(csv.DictReader((OUT / "diagnostic.csv").open()))
    old_rows = list(csv.DictReader((PRIOR / "diagnostic.csv").open()))
    per_case = {}
    for row, old, new_line, old_line in zip_longest(rows, old_rows,
            (OUT / "diagnostic_moves.jsonl").open(), (PRIOR / "diagnostic_moves.jsonl").open()):
        assert row is not None and old is not None and new_line is not None and old_line is not None
        new, saved = json.loads(new_line), json.loads(old_line)
        for key in ("case", "hash", "size", "dependency", "extracted"):
            assert row[key] == old[key]
        for key in ("case", "hash", "ids"):
            assert new[key] == saved[key]
        for key in ("case", "hash"):
            assert new[key] == row[key]
        current = len((ROOT / "results/out/v026_late_start_lns" / row["case"]).read_text().splitlines())
        assert current == int(row["current_T"])
        primary = saved["primary"]
        skipped = primary is not None and len(primary) <= current
        expected = primary if skipped else saved["chosen"]
        assert new["chosen"] == expected
        assert int(row["alternate_skipped"]) == skipped
        assert int(row["alternate_started"]) == int(row["extracted"])-skipped
        outcome = -1 if not int(row["extracted"]) else 3 if primary is None else 0 if len(primary)<current else 1 if len(primary)==current else 2
        assert int(row["outcome"]) == outcome
        assert int(row["chosen_ok"]) == (expected is not None)
        assert int(row["chosen_T"]) == (len(expected) if expected is not None else -1)
        for key in ("alternate_selected", "rescued", "raw_saved"):
            assert int(row[key]) == (0 if skipped else int(old[key]))
        c = per_case.setdefault(row["case"], Counter())
        c["candidates"] += 1
        for key in ("extracted", "alternate_skipped", "alternate_started", "alternate_selected", "rescued", "raw_saved"):
            c[key] += int(row[key])
        if expected is not None:
            validate(row["case"], expected)
            c["chosen_valid"] += 1
            c["changed_results"] += expected != saved["chosen"]
            c["raw_loss"] += len(expected)-len(saved["chosen"])
        if outcome >= 0:
            c[("first_shorter", "first_equal", "first_longer", "first_failed")[outcome]] += 1
    assert len(per_case) == 100
    total, normal = Counter(), Counter()
    for name, c in per_case.items():
        total.update(c)
        if name != "0000.txt":
            normal.update(c)
    assert normal["alternate_skipped"] == 1071
    assert normal["changed_results"] == normal["raw_loss"] == 2
    assert total["alternate_selected"] > 0 and total["rescued"] > 0
    source = ROOT / "src/bin/v029_selective_two_order_lns.cpp"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == audit["solver_sha256"]
    result = {"verified_cases": 100, "total": dict(total), "generated_99": dict(normal),
              "solver_sha256": audit["solver_sha256"], "cases": per_case}
    (OUT / "diagnostic_check.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps({k: v for k, v in result.items() if k != "cases"}, indent=2))


if __name__ == "__main__":
    main()
