#!/usr/bin/env python3
"""Replay saved outputs and apply the preregistered 0.3% score threshold."""
import hashlib
import json
from pathlib import Path

from summarize_v010 import latest_run
from summarize_v021 import compare, read_case

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v025_fast_math"
BIN = "v025_fast_math"
BASE = "v022_dependency_lns"
BASE_RUN = "20260927T155946+0900_v022_dependency_lns_51c972"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    audit = json.loads((OUT / "static_verification.json").read_text())
    for name, expected in audit["source_sha256"].items():
        assert digest(ROOT / f"src/bin/{name}.cpp") == expected, "Solver changed after audit"
    assert digest(ROOT / "src/bin/v000_template.cpp") == audit["template"]["sha256"]
    assert {p.name: digest(p) for p in (ROOT / "tools/in").glob("*.txt")} == audit["input_sha256"]
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, "fast_math")
    prior = [record for record in records if record["run_id"] == BASE_RUN]
    assert len(current) == len(prior) == 100
    assert len({record["run_id"] for record in records if record["bin"] == BIN}) == 1
    assert {record["case_name"] for record in current} == {record["case_name"] for record in prior}
    assert all(record["local"] and record["input_dir"] == "tools/in" for record in current + prior)
    cases = [read_case(record) for record in current]
    parent_cases = [read_case(record) for record in prior]
    assert all(case["verified"] for case in parent_cases), "Baseline output validation failed"
    baselines = {BASE: {record["case_name"]: record for record in prior}}
    totals = compare(cases, baselines)
    parent_totals = compare(parent_cases, {})
    base_score = sum(record["score"] for record in prior)
    full_scores = all(record["score"] is not None and record["status"] == "ok" for record in current)
    score = sum(record["score"] for record in current) if full_scores else None
    degradation = (score - base_score) / base_score * 100 if full_scores else None
    threshold_triggered = (score - base_score) * 1000 >= base_score * 3 if full_scores else True
    quality_pass = (totals["verified_cases"] == totals["cases_under_2000_ms"] == 100
                    and totals["error_count"] == 0)
    mechanism_pass = (audit["code_generation"][BIN]["fast_math_attribute_lines"] > 0
                      and all(item["only_pragma_differs_after_filename_normalization"]
                              for item in audit["preprocessing"].values())
                      and totals["counts_sum"].get("lns_attempts", 0) > 0
                      and totals["counts_sum"].get("lns_accepted", 0) > 0)
    result = {
        "bin": BIN, "run_id": current[0]["run_id"], "baseline_run_id": BASE_RUN,
        "solver_sha256": audit["source_sha256"][BIN],
        "mechanism_pass": mechanism_pass, "quality_pass": quality_pass,
        "score_degradation_percent": degradation, "threshold_percent": 0.3,
        "threshold_triggered": threshold_triggered,
        "retain_fast_math": quality_pass and mechanism_pass and not threshold_triggered,
        "absolute_score": score, "baseline_absolute_score": base_score,
        "all_100": totals, "parent_all_100": parent_totals,
        "generated_99": compare([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": compare([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "cases": cases,
    }
    (OUT / "summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str((OUT / "summary.json").relative_to(ROOT)),
        "retain_fast_math": result["retain_fast_math"],
        "quality_pass": quality_pass, "mechanism_pass": mechanism_pass,
        "absolute_score": score, "baseline_absolute_score": base_score,
        "score_degradation_percent": degradation, "threshold_triggered": threshold_triggered,
        "verified_cases": totals["verified_cases"], "error_count": totals["error_count"],
        "max_elapsed_ms": totals["max_elapsed_ms"], "comparisons": totals["comparisons"],
        "lns_attempts": totals["counts_sum"].get("lns_attempts"),
        "parent_lns_attempts": parent_totals["counts_sum"].get("lns_attempts"),
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
