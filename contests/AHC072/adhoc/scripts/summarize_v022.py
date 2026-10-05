#!/usr/bin/env python3
"""Replay saved v022 outputs and check the preregistered criteria; no solver runs."""

import hashlib
import json
from pathlib import Path

from summarize_v021 import compare, latest_run, read_case

ROOT = Path(__file__).resolve().parents[2]
BIN = "v022_dependency_lns"
LABEL = "dependency_lns"
BASELINE = "v021_paired_event_lns"


def group(cases, baseline):
    result = compare(cases, {BASELINE: baseline})
    keys = ("generated", "multistep", "attempts", "extracted", "completed", "accepted")
    result["dependency_mechanism_passed"] = all(
        result["counts_sum"].get("lns_dependency_" + key, 0) > 0 for key in keys
    )
    for case in cases:
        c = case["counts"]
        d = {k.removeprefix("lns_dependency_"): v for k, v in c.items() if k.startswith("lns_dependency_")}
        assert 0 <= d["attempts"] <= d["generated"] <= d["selected"]
        assert d["generated"] == d["attempts"] + d["cooldown"]
        assert 0 <= d["multistep"] <= d["generated"]
        assert d["extracted"] + d["extract_failed"] <= d["attempts"]
        assert d["completed"] + d["insert_failed"] <= d["extracted"]
        assert 0 <= d["improvements"] <= d["accepted"] <= d["completed"]
        assert 0 <= d["saved"] <= c["lns_regular_saved"]
        assert 0 <= d["current_improvements"] <= d["accepted"]
    return result


def main():
    audit = json.loads((ROOT / "adhoc/v022_audit/static_verification.json").read_text())
    digest = hashlib.sha256((ROOT / f"src/bin/{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == audit["solver_sha256"], "Solver changed after the pre-evaluation audit"
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = latest_run(records, BASELINE, "paired_event_lns")
    assert prior[0]["run_id"] == "20260927T093556+0900_v021_paired_event_lns_987039"
    baseline = {r["case_name"]: r for r in prior}
    assert set(baseline) == {r["case_name"] for r in current}
    cases = [read_case(r) for r in current]
    result = {
        "bin": BIN, "parents": ["v021"], "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "baseline_run_id": prior[0]["run_id"], "solver_sha256": digest,
        "all_100": group(cases, baseline),
        "generated_99": group([c for c in cases if c["case_name"] != "0000.txt"], baseline),
        "case0000": group([c for c in cases if c["case_name"] == "0000.txt"], baseline),
        "cases": cases,
    }
    all_cases = result["all_100"]
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == 0
        and result["generated_99"]["dependency_mechanism_passed"]
        and all_cases["comparisons"][BASELINE]["delta_T"] < 0
        and result["case0000"]["total_T"] <= 43
    )
    prior_cases = {r["case_name"]: read_case(r) for r in prior}
    result["pre_lns_differences"] = {
        "cases": sum(c["counts"]["pre_lns_ops"] != prior_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
        "total": sum(c["counts"]["pre_lns_ops"] - prior_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
    }
    result["baseline_counts_sum"] = {
        key: sum(c["counts"].get(key, 0) for c in prior_cases.values())
        for key in ("lns_attempts", "lns_improvements", "lns_saved")
    }
    result["baseline_times_ms_mean"] = {
        key: sum(c["times_ms"].get(key, 0) for c in prior_cases.values()) / len(prior_cases)
        for key in ("construction", "temporal_lns", "lns_candidate_build")
    }
    path = ROOT / "adhoc/v022_evaluation_summary.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(path.relative_to(ROOT)), "run_id": result["run_id"], "adopt": result["adopt"],
        "mean_T": all_cases["mean_T"], "total_T": all_cases["total_T"],
        "comparisons": all_cases["comparisons"], "verified_cases": all_cases["verified_cases"],
        "max_elapsed_ms": all_cases["max_elapsed_ms"], "error_count": all_cases["error_count"],
        "mechanism_passed": result["generated_99"]["dependency_mechanism_passed"],
        "pre_lns_differences": result["pre_lns_differences"],
        "dependency_counts": {k: v for k, v in all_cases["counts_sum"].items() if k.startswith("lns_dependency_")},
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
