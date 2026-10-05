#!/usr/bin/env python3
"""Verify saved mixed-tree results and compare with v013 without running solvers."""

import hashlib
import json
from pathlib import Path

from summarize_v010 import latest_run
from summarize_v012 import read_case, summarize_cases


ROOT = Path(__file__).resolve().parents[2]
BIN = "v015_mixed_tree"
LABEL = "pro_mixed_tree_port"
BASELINE = "v013_unified"
EXPECTED_SHA256 = "dc9411f97e1d28a2bfba2a0a51d40b1aa989db0fbb77c49df593d8db3cb8fa30"
SOURCE_SHA256 = "f10b4b2998e4d31e9537438637de0ac61a553bb5c06da7bd3573272c21334bcd"


def compare(cases, baseline):
    result = summarize_cases(cases, {BASELINE: baseline})
    sums = result["counts_sum"]
    result["mixed_mechanism_passed"] = all(sums.get(key, 0) > 0 for key in (
        "mixed_decoded", "mixed_routes", "mixed_completed", "mixed_trials",
    ))
    result["phase_differences"] = {
        key: sum(case["counts"].get(key, 0) - baseline[case["case_name"]]["counts"].get(key, 0)
                 for case in cases)
        for key in ("initial_best_ops", "pre_branch_ops", "branch_saved", "local_saved", "joint_saved", "final_ops")
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
    prior = latest_run(records, BASELINE, "pro_unified_refactor")
    assert len(prior) == 100 and all(record["status"] == "ok" and record["local"] for record in prior)
    assert {record["case_name"] for record in prior} == {record["case_name"] for record in current}
    baseline = {record["case_name"]: read_case(record) for record in prior}
    assert all(case["verified"] for case in baseline.values())
    cases = [read_case(record) for record in current]
    for case in cases:
        if not case["verified"]:
            continue
        counts = case["counts"]
        assert 0 <= counts["mixed_completed"] <= counts["mixed_decoded"]
        assert counts["mixed_completed"] <= counts["mixed_routes"]
        assert counts["mixed_trials"] == sum(counts[key] for key in (
            "mixed_merges", "mixed_splits", "mixed_repartitions",
        ))
    result = {
        "bin": BIN, "parent": BASELINE, "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "baseline_run_id": prior[0]["run_id"],
        "original_source_sha256": SOURCE_SHA256, "solver_sha256": digest,
        "all_100": compare(cases, baseline),
        "generated_99": compare([case for case in cases if case["case_name"] != "0000.txt"], baseline),
        "case0000": compare([case for case in cases if case["case_name"] == "0000.txt"], baseline),
        "cases": cases,
    }
    all_cases, generated = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == 100 and all_cases["cases_under_2000_ms"] == 100
        and generated["mixed_mechanism_passed"]
        and all_cases.get("comparisons", {}).get(BASELINE, {}).get("delta_T", 0) < 0
    )
    output = ROOT / "adhoc/v015_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)), "run_id": result["run_id"],
        "adopt": result["adopt"], "verified_cases": all_cases["verified_cases"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"), "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "generated_99_mechanism_passed": generated["mixed_mechanism_passed"],
        "generated_99_mixed": {
            key: {"total": value, "cases": generated["cases_positive"][key]}
            for key, value in generated["counts_sum"].items() if key.startswith("mixed_")
        },
        "phase_differences": all_cases["phase_differences"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
