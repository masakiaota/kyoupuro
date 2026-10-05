#!/usr/bin/env python3
"""Verify one frozen v029 evaluation and its preregistered gate experiment."""
import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import latest_run, read_case
from summarize_v026 import group as late_start_group
from summarize_v028 import group as two_order_group

ROOT = Path(__file__).resolve().parents[2]
BIN = "v029_selective_two_order_lns"
PARENT = "v028_two_order_lns"
REFERENCE = "v026_late_start_lns"
LABEL = "selective_two_order_lns"
PREFIX = "lns_two_order_"
OUTCOMES = ("shorter", "equal", "longer", "failed")


def group(cases, baselines):
    result = two_order_group(cases, baselines)
    c = result["counts_sum"]
    result["gate_mechanism_passed"] = result["two_order_mechanism_passed"] and c[PREFIX+"alternate_skipped"] > 0
    result["gate_active_cases"] = sum(case["counts"][PREFIX+"alternate_skipped"] > 0 for case in cases)
    for case in cases:
        c, t = case["counts"], case["times_ms"]
        z = {k.removeprefix(PREFIX): v for k, v in c.items() if k.startswith(PREFIX)}
        assert z["alternate_skipped"] == z["first_shorter_count"]+z["first_equal_count"]
        assert sum(z[f"first_{o}_count"] for o in OUTCOMES[:3]) == z["primary_finished"]
        assert z["first_failed_count"] == z["primary_failed"]
        assert z["alternate_started"]+z["alternate_skipped"] == z["primary_finished"]+z["primary_failed"]
        for key in ("alternate_started", "alternate_selected", "accepted", "improvements", "saved",
                    "alternate_accepted", "alternate_improvements", "alternate_saved"):
            assert sum(z[f"first_{o}_{key}"] for o in OUTCOMES) == z[key]
        for o in OUTCOMES:
            p = f"first_{o}_"
            assert z[p+"alternate_started"] == (z[p+"count"] if o in OUTCOMES[2:] else 0)
            assert z[p+"alternate_improvements"] <= z[p+"improvements"] <= z[p+"accepted"] <= z[p+"count"]
            assert z[p+"alternate_accepted"] <= z[p+"alternate_selected"] <= z[p+"alternate_started"]
            assert z[p+"alternate_improvements"] <= z[p+"alternate_accepted"] <= z[p+"accepted"]
            assert z[p+"alternate_saved"] <= z[p+"saved"]
            if o in OUTCOMES[:2]:
                assert t[PREFIX+p+"alternate_search"] == 0
        for branch in ("primary", "alternate"):
            subtotal = sum(t[PREFIX+f"first_{o}_{branch}_search"] for o in OUTCOMES)
            if branch == "primary":
                subtotal += t[PREFIX+"unclassified_primary_search"]
            assert abs(subtotal-t[PREFIX+branch+"_search"]) <= 0.004
    return result


def main():
    audit = json.loads((ROOT / "adhoc/v029_audit/static_verification.json").read_text())
    diagnostic = json.loads((ROOT / "adhoc/v029_audit/diagnostic_check.json").read_text())
    for name, key in ((BIN, "solver_sha256"), (PARENT, "parent_sha256")):
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    for name, digest in audit["input_sha256"].items():
        assert hashlib.sha256((ROOT / "tools/in" / name).read_bytes()).hexdigest() == digest
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = {PARENT: latest_run(records, PARENT, "two_order_lns"),
             REFERENCE: latest_run(records, REFERENCE, "late_start_lns")}
    assert prior[PARENT][0]["run_id"] == "20260927T184531+0900_v028_two_order_lns_c6311b"
    assert prior[REFERENCE][0]["run_id"] == "20260927T172632+0900_v026_late_start_lns_973c99"
    baselines = {name: {r["case_name"]: r for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(r["local"] and r["status"] == "ok" for r in baseline.values())
    cases = [read_case(r) for r in current]
    parent_cases = {r["case_name"]: read_case(r) for r in prior[PARENT]}
    reference_cases = [read_case(r) for r in prior[REFERENCE]]
    result = {
        "bin": BIN, "parents": ["v028"], "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "solver_sha256": audit["solver_sha256"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in prior.items()},
        "diagnostic_verification": {k: v for k, v in diagnostic.items() if k != "cases"},
        "all_100": group(cases, baselines),
        "generated_99": group([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": group([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "parent_generated_99": two_order_group([c for name, c in parent_cases.items() if name != "0000.txt"], {}),
        "reference_generated_99": late_start_group([c for c in reference_cases if c["case_name"] != "0000.txt"], {}),
        "cases": cases,
    }
    all_cases, normal, parent_normal = (result[k] for k in ("all_100", "generated_99", "parent_generated_99"))
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == all_cases["counts_sum"].get("lns_invalid_candidates", 0) == 0
        and diagnostic["verified_cases"] == 100 and normal["gate_mechanism_passed"]
        and all_cases["comparisons"][PARENT]["delta_T"] < 0
        and result["case0000"]["total_T"] <= 43
    )
    result["pre_lns_differences"] = {
        "cases": sum(c["counts"]["pre_lns_ops"] != parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
        "total": sum(c["counts"]["pre_lns_ops"]-parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
    }
    n, p = normal["counts_sum"], parent_normal["counts_sum"]
    result["relative_changes_percent"] = {
        "lns_attempts": 100*(n["lns_attempts"]/p["lns_attempts"]-1),
        "loops": 100*(normal["loops_including_dependency_skips"]/parent_normal["loops_including_dependency_skips"]-1),
        "alternate_started": 100*(n[PREFIX+"alternate_started"]/p[PREFIX+"alternate_started"]-1),
        "alternate_time": 100*(normal["mean_times_ms"][PREFIX+"alternate_search"]/parent_normal["mean_times_ms"][PREFIX+"alternate_search"]-1),
        "skipped_share_of_two_order_attempts": 100*n[PREFIX+"alternate_skipped"]/n[PREFIX+"attempts"],
    }
    (ROOT / "adhoc/v029_evaluation_summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2)+"\n")
    with (ROOT / "adhoc/v029_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(("case", "v026_T", "v028_T", "v029_T", "delta_vs_v028", "pre_lns_delta", "v029_attempts",
                         "two_order_attempts", "alternate_skipped", "alternate_started", "alternate_selected", "alternate_best_saved", "elapsed_ms"))
        for case in sorted(cases, key=lambda c: c["case_name"]):
            p, c = parent_cases[case["case_name"]], case["counts"]
            writer.writerow((case["case_name"], baselines[REFERENCE][case["case_name"]]["score"], p["score"], case["score"], case["score"]-p["score"],
                             c["pre_lns_ops"]-p["counts"]["pre_lns_ops"], c["lns_attempts"],
                             c[PREFIX+"attempts"], c[PREFIX+"alternate_skipped"], c[PREFIX+"alternate_started"],
                             c[PREFIX+"alternate_selected"], c[PREFIX+"alternate_saved"], case["elapsed"]))
    print(json.dumps({
        "run_id": result["run_id"], "adopt": result["adopt"],
        "mean_T": all_cases["mean_T"], "total_T": all_cases["total_T"],
        "comparisons": all_cases["comparisons"], "verified_cases": all_cases["verified_cases"],
        "max_elapsed_ms": all_cases["max_elapsed_ms"], "error_count": all_cases["error_count"],
        "mechanism_passed": normal["gate_mechanism_passed"],
        "alternate_improved_cases": normal["alternate_improved_cases"],
        "pre_lns_differences": result["pre_lns_differences"],
        "relative_changes_percent": result["relative_changes_percent"],
        "two_order_counts": {k: v for k, v in normal["counts_sum"].items() if k.startswith(PREFIX)},
        "two_order_times_ms": {k: v for k, v in normal["mean_times_ms"].items() if k.startswith(PREFIX)},
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
