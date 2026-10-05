#!/usr/bin/env python3
"""Verify the saved LOCAL evaluation; do not run or alter either solver."""

import hashlib
import json
from pathlib import Path

from summarize_v010 import latest_run
from summarize_v012 import read_case, summarize_cases


ROOT = Path(__file__).resolve().parents[2]
BIN = "v017_timer_origin"
LABEL = "timer_origin_local"
BASELINE = "v015_mixed_tree"


def main():
    audit = ROOT / "adhoc/v017_timer_audit"
    manifest = json.loads((audit / "manifest.json").read_text())
    verification = json.loads((audit / "static_verification.json").read_text())
    for name, key in ((BIN, "solver_sha256"), (BASELINE, "parent_sha256")):
        source = ROOT / "src/bin" / f"{name}.cpp"
        assert hashlib.sha256(source.read_bytes()).hexdigest() == manifest[key]
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    matching = [r for r in records if r["bin"] == BIN]
    assert len({r["run_id"] for r in matching}) == 1
    current = latest_run(records, BIN, LABEL)
    prior = latest_run(records, BASELINE, "pro_mixed_tree_port")
    for run in (current, prior):
        assert len(run) == len({r["case_name"] for r in run}) == 100
        assert all(r["local"] and r["input_dir"] == "tools/in" for r in run)
    baseline = {r["case_name"]: read_case(r) for r in prior}
    assert all(c["verified"] for c in baseline.values())
    cases = [read_case(r) for r in current]
    summary = {
        "bin": BIN,
        "label": LABEL,
        "run_id": current[0]["run_id"],
        "executed_at": current[0]["executed_at"],
        "baseline_run_id": prior[0]["run_id"],
        "solver_sha256": manifest["solver_sha256"],
        "static_verification": verification,
        "all_100": summarize_cases(cases, {BASELINE: baseline}),
        "generated_99": summarize_cases([c for c in cases if c["case_name"] != "0000.txt"], {BASELINE: baseline}),
        "case0000": summarize_cases([c for c in cases if c["case_name"] == "0000.txt"], {BASELINE: baseline}),
        "submission_performance_evaluated": False,
        "memory_evaluated": False,
        "comparison_scope": "LOCAL source is unchanged. Score differences do not measure the non-LOCAL timer change.",
        "cases": cases,
    }
    all_cases = summary["all_100"]
    summary["adopt_for_submission_comparison"] = (
        verification["local_directives_identical"]
        and verification["non_local_original_clock_order"]
        and verification["both_builds_succeeded"]
        and all_cases["verified_cases"] == 100
        and all_cases["cases_under_2000_ms"] == 100
    )
    output = ROOT / "adhoc/v017_evaluation_summary.json"
    output.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output),
        "adopt_for_submission_comparison": summary["adopt_for_submission_comparison"],
        "verified_cases": all_cases["verified_cases"],
        "mean_T": all_cases.get("mean_T"),
        "comparisons": all_cases.get("comparisons"),
        "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "submission_performance_evaluated": False,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
