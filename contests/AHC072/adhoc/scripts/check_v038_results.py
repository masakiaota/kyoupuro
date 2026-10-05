#!/usr/bin/env python3
"""Verify saved diagnostic/evaluation outputs; never invoke a solver."""
import csv
import hashlib
import json
from pathlib import Path
import re
import sys

from check_v028_two_orders import validate

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v038_audit"
BIN = "v038_alternating_polish"
PARENT_RUN = "20260928T020458+0900_v037_pair_transfer_compression_6086dd"
ERRORS = ("baseline_recovery", "final_recovery", "construction_errors", "lns_errors", "lns_invalid_candidates")


def verify_source():
    audit = json.loads((AUDIT / "static_verification.json").read_text())
    for name, key in ((BIN, "solver_sha256"), ("v037_pair_transfer_compression", "parent_sha256")):
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    for name, digest in audit["input_sha256"].items():
        assert hashlib.sha256((ROOT / "tools/in" / name).read_bytes()).hexdigest() == digest
    return audit


def verify_output(case, path):
    moves = []
    for line in path.read_text().splitlines():
        i, j, k, d, l = line.split()
        moves.append((int(i), int(j), int(k), d, int(l)))
    validate(case, moves)
    return len(moves)


def diagnostic():
    audit = verify_source()
    with (AUDIT / "diagnostic.csv").open() as f:
        rows = list(csv.DictReader(f))
    assert len(rows) == len({r["case"] for r in rows}) == 100
    for r in rows:
        r.update({k: int(v) for k, v in r.items() if k != "case" and not k.endswith("_us")})
        for kind in ("route_once", "route_only", "alternating"):
            actual = verify_output(r["case"], AUDIT / kind / r["case"])
            assert actual == r[kind + "_T"] <= r["input_T"]
        assert r["input_T"] - r["alternating_T"] == r["route_saved"] + r["pair_saved"]
        assert r["rounds"] > 0
    mechanism = json.loads((AUDIT / "mechanism.json").read_text())
    result = {
        "solver_sha256": audit["solver_sha256"], "verified_cases": 100,
        "verified_outputs": 300, "mechanism": mechanism, "cases": rows,
        "proceed_to_evaluation": mechanism["fixed_checks"] == 6 and mechanism["alternating_saved"] > 0,
    }
    (AUDIT / "diagnostic_verification.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "cases"}, indent=2))


def log_values(path):
    text = path.read_text()
    counts = {k: int(v) for k, v in re.findall(r"\[summary.count\] ([^=]+)=(-?\d+)", text)}
    times = {k: float(v) for k, v in re.findall(r"\[summary.time_ms\] ([^=]+)=([\d.]+)", text)}
    return counts, times, [line for line in text.splitlines() if "diagnostic:" in line]


def evaluation():
    audit = verify_source()
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = [r for r in records if r["bin"] == BIN]
    parent = {r["case_name"]: r for r in records if r["run_id"] == PARENT_RUN}
    assert len(current) == len(parent) == 100
    assert len({r["run_id"] for r in current}) == 1
    assert {r["case_name"] for r in current} == parent.keys()
    cases = []
    for r in sorted(current, key=lambda r: r["case_name"]):
        assert r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in"
        path = ROOT / r["stdout_path"]
        T = verify_output(r["case_name"], path)
        c, t, diagnostics = log_values(path.with_suffix(path.suffix + ".err"))
        assert T == r["score"] == c["T"] == c["final_ops"] == c["validated_moves"] and c["E"] == 0
        assert c["pair_transfer_saved"] == c["pre_pair_ops"] - c["post_pair_ops"] + c["final_polish_pair_saved"]
        assert c["final_polish_saved"] == c["post_pair_ops"] - T
        assert c["final_polish_saved"] == c["final_polish_route_saved"] + c["final_polish_pair_saved"]
        assert c["final_polish_deadlines"] + c["final_polish_converged"] == 1
        assert c["final_polish_route_calls"] >= c["final_polish_pair_calls"] == c["final_polish_rounds"]
        assert c["pre_pair_ops"] - T == c["pair_transfer_saved"] + c["final_polish_route_saved"]
        assert c["pair_transfer_saved"] == c["pair_transfer_merges"] + c["pair_transfer_cancellations"] >= 0
        assert c["lns_saved"] == c["pre_lns_ops"] - c["pre_pair_ops"]
        assert c["lns_saved"] == c["lns_initial_shortcut_saved"] + c["lns_initial_reorder_saved"] + c["lns_best_saved"]
        assert t["search_limit"] == 1544.0
        p = parent[r["case_name"]]
        pc, _, _ = log_values((ROOT / p["stdout_path"]).with_suffix(".txt.err"))
        cases.append({**r, "parent_T": p["score"], "delta_T": T - p["score"],
                      "post_pair_delta_T": c["post_pair_ops"] - p["score"],
                      "pre_lns_delta_T": c["pre_lns_ops"] - pc["pre_lns_ops"],
                      "counts": c, "times_ms": t, "diagnostics": diagnostics})
    T = sum(r["score"] for r in cases)
    parent_T = sum(r["score"] for r in parent.values())
    errors = {k: sum(r["counts"][k] for r in cases) for k in ERRORS}
    saved = sum(r["counts"]["final_polish_saved"] for r in cases)
    diagnostic_result = json.loads((AUDIT / "diagnostic_verification.json").read_text())
    result = {
        "run_id": current[0]["run_id"], "parent_run_id": PARENT_RUN,
        "solver_sha256": audit["solver_sha256"], "verified_cases": len(cases),
        "total_T": T, "parent_T": parent_T, "delta_T": T - parent_T,
        "delta_percent": 100 * (T / parent_T - 1),
        "wins": sum(r["delta_T"] < 0 for r in cases),
        "draws": sum(r["delta_T"] == 0 for r in cases),
        "losses": sum(r["delta_T"] > 0 for r in cases),
        "case0000_T": next(r["score"] for r in cases if r["case_name"] == "0000.txt"),
        "mean_elapsed_ms": sum(r["elapsed"] for r in cases) / len(cases),
        "max_elapsed_ms": max(r["elapsed"] for r in cases),
        "errors": errors, "diagnostics_count": sum(len(r["diagnostics"]) for r in cases),
        "post_pair_T": T + saved, "final_polish_saved": saved,
        "polish_shortened_cases": sum(r["counts"]["final_polish_saved"] > 0 for r in cases),
        "polish_counts": {k: sum(r["counts"][k] for r in cases) for k in cases[0]["counts"] if k.startswith("final_polish_")},
        "polish_mean_ms": sum(r["times_ms"]["final_polish"] for r in cases) / len(cases),
        "polish_max_ms": max(r["times_ms"]["final_polish"] for r in cases),
        "pre_lns_delta_T": sum(r["pre_lns_delta_T"] for r in cases),
        "pre_lns_changed_cases": sum(r["pre_lns_delta_T"] != 0 for r in cases),
        "diagnostic_saved_moves": diagnostic_result["mechanism"]["alternating_saved"],
    }
    result["adopt"] = (
        not any(errors.values()) and result["diagnostics_count"] == 0
        and result["max_elapsed_ms"] <= 2000 and result["case0000_T"] <= 43
        and result["delta_T"] < 0 and saved > 0
        and diagnostic_result["verified_cases"] == 100
    )
    with (ROOT / "adhoc/v038_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(("case", "v037_T", "post_pair_T", "v038_T", "delta_T", "polish_saved", "route_saved", "extra_pair_saved", "polish_ms", "elapsed_ms"))
        for r in cases:
            writer.writerow((r["case_name"], r["parent_T"], r["counts"]["post_pair_ops"], r["score"], r["delta_T"],
                             r["counts"]["final_polish_saved"], r["counts"]["final_polish_route_saved"], r["counts"]["final_polish_pair_saved"], r["times_ms"]["final_polish"], r["elapsed"]))
    (ROOT / "adhoc/v038_evaluation_summary.json").write_text(json.dumps({**result, "cases": cases}, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    {"diagnostic": diagnostic, "evaluation": evaluation}[sys.argv[1]]()
