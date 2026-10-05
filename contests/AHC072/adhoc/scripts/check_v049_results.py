#!/usr/bin/env python3
"""Verify frozen transformations and one full evaluation; never run a solver."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v049_audit"
OUT = ROOT / "results/analysis/v049"
BIN = "v049_coupled_routes"
PARENT_RUN = "20260928T092503+0900_v039_exact_board_lns_3c2389"


def verify_sources():
    audit = json.loads((AUDIT / "static_verification.json").read_text())
    for name, key in [(BIN, "solver_sha256"), ("v039_exact_board_lns", "parent_sha256")]:
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    for case, digest in audit["input_sha256"].items():
        assert hashlib.sha256((ROOT / "tools/in" / case).read_bytes()).hexdigest() == digest
    return audit


def diagnostic():
    audit = verify_sources()
    rows = list(csv.DictReader((AUDIT / "diagnostic.csv").open()))
    for row in rows:
        for key in row:
            if key not in ("group", "case"):
                row[key] = float(row[key]) if key == "ms" else int(row[key])
        T = verify_output(row["case"] + ".txt", AUDIT / ("reduced_" + row["group"]) / (row["case"] + ".txt"))
        assert T == row["after"] <= row["before"] and row["before"] - T == row["saved"]
    assert len(rows) == 110 and len({(r["group"], r["case"]) for r in rows}) == 110
    for record in ("000055",):
        verify_output("0005.txt", AUDIT / "witnesses" / (record + ".txt"))
    result = {"solver_sha256": audit["solver_sha256"], "verified_cases": len(rows),
              "mechanism": json.loads((AUDIT / "mechanism.json").read_text()), "groups": {}}
    for group in ("parent", "initial"):
        selected = [r for r in rows if r["group"] == group]
        result["groups"][group] = {"cases": len(selected), "saved": sum(r["saved"] for r in selected),
            "changed": sum(r["saved"] > 0 for r in selected),
            "mean_ms": sum(r["ms"] for r in selected) / len(selected), "max_ms": max(r["ms"] for r in selected)}
    (AUDIT / "diagnostic_verification.json").write_text(json.dumps({**result, "cases": rows}, indent=2) + "\n")
    print(json.dumps(result, indent=2))


def evaluation():
    audit = verify_sources()
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = [r for r in records if r["bin"] == BIN]
    parent = {r["case_name"]: r for r in records if r["run_id"] == PARENT_RUN}
    assert len(current) == len(parent) == 100 and len({r["run_id"] for r in current}) == 1
    cases = []
    for record in sorted(current, key=lambda r: r["case_name"]):
        assert record["status"] == "ok" and record["local"] and record["input_dir"] == "tools/in"
        path = ROOT / record["stdout_path"]
        T = verify_output(record["case_name"], path)
        counts, times, diagnostics = log_values(path.with_suffix(".txt.err"))
        assert T == record["score"] == counts["T"] == counts["final_ops"] == counts["validated_moves"]
        assert counts["E"] == 0 and counts["board_pool_free_at_end"] == 4
        assert T == counts["pre_pair_ops"] - counts["pair_transfer_saved"]
        assert counts["pre_lns_ops"] - counts["pre_pair_ops"] == counts["lns_saved"]
        baseline = parent[record["case_name"]]
        old_counts, _, _ = log_values((ROOT / baseline["stdout_path"]).with_suffix(".txt.err"))
        cases.append({**record, "parent_T": baseline["score"], "delta_T": T - baseline["score"],
                      "pre_lns_delta": counts["pre_lns_ops"] - old_counts["pre_lns_ops"],
                      "parent_attempts": old_counts["lns_attempts"],
                      "counts": counts, "times_ms": times, "diagnostics": diagnostics})
    total = sum(r["score"] for r in cases)
    baseline = sum(r["score"] for r in parent.values())
    counts = {key: sum(r["counts"][key] for r in cases) for key in cases[0]["counts"] if key.startswith("coupled_routes_")}
    errors = {key: sum(r["counts"][key] for r in cases) for key in ERRORS}
    attempts = sum(r["counts"]["lns_attempts"] for r in cases)
    old_attempts = sum(r["parent_attempts"] for r in cases)
    result = {"run_id": current[0]["run_id"], "parent_run_id": PARENT_RUN,
              "solver_sha256": audit["solver_sha256"], "verified_cases": 100,
              "total_T": total, "parent_T": baseline, "delta_T": total - baseline,
              "delta_percent": 100 * (total / baseline - 1),
              "wins": sum(r["delta_T"] < 0 for r in cases), "draws": sum(r["delta_T"] == 0 for r in cases),
              "losses": sum(r["delta_T"] > 0 for r in cases), "case0000_T": cases[0]["score"],
              "normal_99_avg": (total - cases[0]["score"]) / 99,
              "max_elapsed_ms": max(r["elapsed"] for r in cases), "errors": errors,
              "diagnostic_count": sum(len(r["diagnostics"]) for r in cases), "counts": counts,
              "accepted_cases": sum(r["counts"]["coupled_routes_accepted_saved"] > 0 for r in cases),
              "mean_coupled_ms": sum(r["times_ms"]["coupled_routes"] for r in cases) / 100,
              "max_coupled_ms": max(r["times_ms"]["coupled_routes"] for r in cases),
              "attempts": attempts, "parent_attempts": old_attempts,
              "attempts_delta_percent": 100 * (attempts / old_attempts - 1),
              "pre_lns_delta_T": sum(r["pre_lns_delta"] for r in cases),
              "pre_lns_changed_cases": sum(r["pre_lns_delta"] != 0 for r in cases)}
    result["adopt"] = (not any(errors.values()) and result["diagnostic_count"] == 0 and
                       result["max_elapsed_ms"] <= 2000 and result["case0000_T"] <= 43 and
                       total < baseline and counts["coupled_routes_accepted_saved"] > 0)
    OUT.mkdir(parents=True, exist_ok=True)
    with (OUT / "comparison.csv").open("w") as file:
        writer = csv.writer(file)
        writer.writerow(("case", "parent_T", "T", "delta_T", "coupled_saved", "accepted_saved", "best_crossings", "coupled_ms"))
        for r in cases:
            c = r["counts"]
            writer.writerow((r["case_name"], r["parent_T"], r["score"], r["delta_T"], c["coupled_routes_saved"],
                             c["coupled_routes_accepted_saved"], c["coupled_routes_best_crossings"], r["times_ms"]["coupled_routes"]))
    (OUT / "summary.json").write_text(json.dumps({**result, "cases": cases}, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    {"diagnostic": diagnostic, "evaluation": evaluation}[sys.argv[1]]()
