#!/usr/bin/env python3
"""Compare the saved v013 refactoring evaluation with v012; never run solvers."""

import hashlib
import json
from pathlib import Path

from summarize_v010 import latest_run
from summarize_v012 import read_case, summarize_cases


ROOT = Path(__file__).resolve().parents[2]
BIN = "v013_unified"
LABEL = "pro_unified_refactor"
BASELINE = "v012_pruned"
EXPECTED_SHA256 = "c92ab4ccc713828d3ba7f2876f92678c35e40883f75774e4ea1a8770d92793a3"
SOURCE_SHA256 = "d29ad64b1d6f294effd1b3fb1aace559a956b3fcc638e76e03444c8069a8964f"


def compare(cases, baseline):
    result = summarize_cases(cases, {BASELINE: baseline})
    keys = ("pre_branch_ops", "branch_saved", "local_saved", "joint_saved", "final_ops")
    result["phase_differences"] = {
        key: sum(case["counts"].get(key, 0) - baseline[case["case_name"]]["counts"].get(key, 0)
                 for case in cases)
        for key in keys
    }
    return result


def main():
    digest = hashlib.sha256((ROOT / "src/bin" / f"{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == EXPECTED_SHA256, "Solver changed after the pre-evaluation build"
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({record["run_id"] for record in records if record["bin"] == BIN}) == 1
    assert len(current) == len({record["case_name"] for record in current}) == 100
    assert all(record["local"] and record["input_dir"] == "tools/in" for record in current)
    prior = latest_run(records, BASELINE, "pro_pruned_port")
    assert len(prior) == 100 and all(record["status"] == "ok" and record["local"] for record in prior)
    assert {record["case_name"] for record in prior} == {record["case_name"] for record in current}
    baseline = {record["case_name"]: read_case(record) for record in prior}
    assert all(case["verified"] for case in baseline.values())
    cases = [read_case(record) for record in current]
    result = {
        "bin": BIN, "parent": BASELINE, "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "baseline_run_id": prior[0]["run_id"],
        "original_source_sha256": SOURCE_SHA256, "solver_sha256": digest,
        "all_100": compare(cases, baseline),
        "generated_99": compare([case for case in cases if case["case_name"] != "0000.txt"], baseline),
        "cases": cases,
    }
    all_cases, generated = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == 100 and all_cases["cases_under_2000_ms"] == 100
        and generated["mechanism_passed"]
        and all_cases.get("comparisons", {}).get(BASELINE, {}).get("delta_T", 1) <= 0
    )
    output = ROOT / "adhoc/v013_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)), "run_id": result["run_id"],
        "adopt": result["adopt"], "verified_cases": all_cases["verified_cases"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"), "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "generated_99_postprocessing": generated["postprocessing"],
        "generated_99_mechanism_passed": generated["mechanism_passed"],
        "phase_differences": all_cases["phase_differences"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
