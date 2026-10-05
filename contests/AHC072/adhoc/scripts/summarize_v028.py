#!/usr/bin/env python3
"""Verify frozen v028 outputs and judge the preregistered two-order experiment."""
import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import latest_run, read_case
from summarize_v026 import group as late_start_group

ROOT = Path(__file__).resolve().parents[2]
BIN = "v028_two_order_lns"
PARENT = "v026_late_start_lns"
LABEL = "two_order_lns"
PREFIX = "lns_two_order_"


def group(cases, baselines):
    result = late_start_group(cases, baselines)
    c = result["counts_sum"]
    result["two_order_mechanism_passed"] = all(c[PREFIX+k] > 0 for k in (
        "attempts", "alternate_started", "alternate_finished", "alternate_selected", "alternate_accepted"))
    result["alternate_improved_cases"] = sum(case["counts"][PREFIX+"alternate_improvements"] > 0 for case in cases)
    for case in cases:
        c = case["counts"]
        z = {k.removeprefix(PREFIX): v for k, v in c.items() if k.startswith(PREFIX)}
        assert sum(z[f"size_{n}"] for n in (2, 3, 4)) == z["attempts"] == z["primary_started"]
        assert z["alternate_selected"] == z["alternate_finished"] == z["rescued"]+z["shorter"]
        assert z["raw_saved"] >= z["shorter"]
        assert 0 <= z["alternate_improvements"] <= z["alternate_accepted"] <= z["alternate_selected"] <= z["completed"]
        assert 0 <= z["improvements"] <= z["accepted"] <= z["completed"] <= z["attempts"] <= c["lns_attempts"]
        assert z["alternate_improvements"] <= z["improvements"]
        assert z["alternate_accepted"] <= z["accepted"]
        assert z["alternate_saved"] <= z["saved"] <= c["lns_regular_saved"]
        assert 0 <= z["primary_finished"]+z["rescued"]-z["completed"] <= 1
        for branch in ("primary", "alternate"):
            assert 0 <= z[branch+"_finished"]+z[branch+"_failed"] <= z[branch+"_started"]
            assert 0 <= z[branch+"_insert_calls"] <= sum(n*z[f"size_{n}"] for n in (2, 3, 4))
        assert z["alternate_started"] <= z["primary_finished"]+z["primary_failed"]
    return result


def main():
    audit = json.loads((ROOT / "adhoc/v028_audit/static_verification.json").read_text())
    diagnostic = json.loads((ROOT / "adhoc/v028_audit/diagnostic_check.json").read_text())
    for name, key in ((BIN, "solver_sha256"), (PARENT, "parent_sha256")):
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    for name, digest in audit["input_sha256"].items():
        assert hashlib.sha256((ROOT / "tools/in" / name).read_bytes()).hexdigest() == digest
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = {PARENT: latest_run(records, PARENT, "late_start_lns"),
             "v022_dependency_lns": latest_run(records, "v022_dependency_lns", "dependency_lns")}
    assert prior[PARENT][0]["run_id"] == "20260927T172632+0900_v026_late_start_lns_973c99"
    baselines = {name: {r["case_name"]: r for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(r["local"] and r["status"] == "ok" for r in baseline.values())
    cases = [read_case(r) for r in current]
    parent_cases = {r["case_name"]: read_case(r) for r in prior[PARENT]}
    result = {
        "bin": BIN, "parents": ["v026"], "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "solver_sha256": audit["solver_sha256"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in prior.items()},
        "diagnostic_verification": {k: v for k, v in diagnostic.items() if k != "cases"},
        "all_100": group(cases, baselines),
        "generated_99": group([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": group([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "parent_generated_99": late_start_group([c for name, c in parent_cases.items() if name != "0000.txt"], {}),
        "cases": cases,
    }
    all_cases, normal = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == 0 and diagnostic["verified_cases"] == 100
        and normal["two_order_mechanism_passed"]
        and all_cases["comparisons"][PARENT]["delta_T"] < 0
        and result["case0000"]["total_T"] <= 43
    )
    result["pre_lns_differences"] = {
        "cases": sum(c["counts"]["pre_lns_ops"] != parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
        "total": sum(c["counts"]["pre_lns_ops"] - parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
    }
    parent_normal = result["parent_generated_99"]
    result["relative_changes_percent"] = {
        "lns_attempts": 100*(normal["counts_sum"]["lns_attempts"] / parent_normal["counts_sum"]["lns_attempts"] - 1),
        "loops": 100*(normal["loops_including_dependency_skips"] / parent_normal["loops_including_dependency_skips"] - 1),
        "candidate_build_time": 100*(normal["mean_times_ms"]["lns_candidate_build"] / parent_normal["mean_times_ms"]["lns_candidate_build"] - 1),
    }
    output = ROOT / "adhoc/v028_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2)+"\n")
    with (ROOT / "adhoc/v028_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(("case", "v026_T", "v028_T", "delta_T", "pre_lns_delta", "v028_attempts",
                         "two_order_attempts", "alternate_selected", "alternate_best_saved", "elapsed_ms"))
        for case in sorted(cases, key=lambda c: c["case_name"]):
            p, c = parent_cases[case["case_name"]], case["counts"]
            writer.writerow((case["case_name"], p["score"], case["score"], case["score"]-p["score"],
                             c["pre_lns_ops"]-p["counts"]["pre_lns_ops"], c["lns_attempts"],
                             c[PREFIX+"attempts"], c[PREFIX+"alternate_selected"],
                             c[PREFIX+"alternate_saved"], case["elapsed"]))
    print(json.dumps({
        "run_id": result["run_id"], "adopt": result["adopt"],
        "mean_T": all_cases["mean_T"], "total_T": all_cases["total_T"],
        "comparisons": all_cases["comparisons"], "verified_cases": all_cases["verified_cases"],
        "max_elapsed_ms": all_cases["max_elapsed_ms"], "error_count": all_cases["error_count"],
        "mechanism_passed": normal["two_order_mechanism_passed"],
        "alternate_improved_cases": normal["alternate_improved_cases"],
        "pre_lns_differences": result["pre_lns_differences"],
        "relative_changes_percent": result["relative_changes_percent"],
        "two_order_counts": {k: v for k, v in normal["counts_sum"].items() if k.startswith(PREFIX)},
        "two_order_times_ms": {k: v for k, v in normal["mean_times_ms"].items() if k.startswith(PREFIX)},
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
