#!/usr/bin/env python3
"""Verify and compare saved v012 results without executing any solver."""

import hashlib
import json
from pathlib import Path

from analyze_v010_structure import replay
from summarize_v010 import COUNT, TIME, latest_run, summarize


ROOT = Path(__file__).resolve().parents[2]
BIN = "v012_pruned"
LABEL = "pro_pruned_port"
EXPECTED_SHA256 = "25a3adcaf464141e473b7fd7b2902846b347f5f9fbf6e72fa6254ef7f06c5d06"
SOURCE_SHA256 = "ba3d67d4a125467cc5aaeb2dba03e2bb2f31471208f0455a75ff27ce372d4055"
BASELINES = {
    "v011_network_regroup": "b14_network_regroup",
    "v010_scaffold_chain": "pro_scaffold_chain_port",
}


def read_case(record):
    path = ROOT / record["stdout_path"]
    text = path.with_name(path.name + ".err").read_text()
    counts = {key: int(value) for key, value in COUNT.findall(text)}
    times = {key: float(value) for key, value in TIME.findall(text)}
    lines = len(path.read_text().splitlines())
    verified = (
        record["status"] == "ok" and counts.get("E") == 0
        and counts.get("T") == counts.get("final_ops")
        == counts.get("validated_moves") == record["score"] == lines
        and lines <= 100000
    )
    if verified:
        structure = replay(ROOT / "tools/in" / record["case_name"], path)
        assert structure["counts"]["T"] == lines
        assert structure["counts"]["M"] == counts["delivered_slimes"]
        assert counts["branch_saved"] == counts["pre_branch_ops"] - counts["pre_local_ops"]
        assert counts["local_saved"] + counts["joint_saved"] == counts["pre_local_ops"] - lines
        assert counts["joint_saved"] == counts["predecessor_saved"] + counts["successor_saved"]
        assert times["search_limit"] == 1472.0
    return {**record, "counts": counts, "times_ms": times, "verified": verified}


def summarize_cases(cases, baselines):
    result = summarize(cases, baselines)
    sums = result["counts_sum"]
    result["mechanism_passed"] = all(sums.get(key, 0) > 0 for key in (
        "branch_trials", "local_windows", "joint_trials",
    ))
    result["postprocessing"] = {
        "branch_saved": sums.get("branch_saved", 0),
        "local_saved": sums.get("local_saved", 0),
        "joint_saved": sums.get("joint_saved", 0),
        "total_saved": sum(sums.get(key, 0) for key in (
            "branch_saved", "local_saved", "joint_saved",
        )),
        "improved_cases": sum(
            c["counts"].get("pre_branch_ops", 0) > c["counts"].get("final_ops", 0)
            for c in cases if c["verified"]
        ),
    }
    return result


def main():
    digest = hashlib.sha256((ROOT / "src/bin" / f"{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == EXPECTED_SHA256, "Solver changed after the pre-evaluation build"
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    matching = [record for record in records if record["bin"] == BIN]
    assert len({record["run_id"] for record in matching}) == 1, "Expected exactly one evaluation"
    assert len(current) == len({record["case_name"] for record in current}) == 100
    assert all(record["local"] and record["input_dir"] == "tools/in" for record in current)
    baseline_runs = {name: latest_run(records, name, label) for name, label in BASELINES.items()}
    baselines = {name: {record["case_name"]: record for record in run} for name, run in baseline_runs.items()}
    for baseline in baselines.values():
        assert set(baseline) == {record["case_name"] for record in current}
        assert all(record["status"] == "ok" and record["local"] for record in baseline.values())
    cases = [read_case(record) for record in current]
    result = {
        "bin": BIN, "label": LABEL, "run_id": current[0]["run_id"],
        "executed_at": current[0]["executed_at"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in baseline_runs.items()},
        "original_source_sha256": SOURCE_SHA256, "solver_sha256": digest,
        "all_100": summarize_cases(cases, baselines),
        "generated_99": summarize_cases([case for case in cases if case["case_name"] != "0000.txt"], baselines),
        "cases": cases,
    }
    all_cases, generated = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == 100 and all_cases["cases_under_2000_ms"] == 100
        and generated["mechanism_passed"]
        and all_cases.get("comparisons", {}).get("v011_network_regroup", {}).get("delta_T", 0) < 0
    )
    output = ROOT / "adhoc/v012_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)), "run_id": result["run_id"],
        "adopt": result["adopt"], "verified_cases": all_cases["verified_cases"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"),
        "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "generated_99_postprocessing": generated["postprocessing"],
        "generated_99_mechanism_passed": generated["mechanism_passed"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
