#!/usr/bin/env python3
"""Verify and compare saved B-14 evaluation records; do not execute any solver."""

import hashlib
import json
from pathlib import Path

from analyze_v010_structure import replay
from summarize_v010 import COUNT, TIME, latest_run, summarize


ROOT = Path(__file__).resolve().parents[2]
BIN = "v011_network_regroup"
BASELINE = "v010_scaffold_chain"
EXPECTED_SHA256 = "9b98639dd778a5955b554e52b721cbc4a5a750dff1401c328dd82104aedeb1d8"


def read_case(record, verify_output):
    path = ROOT / record["stdout_path"]
    text = path.with_name(path.name + ".err").read_text()
    counts = {key: int(value) for key, value in COUNT.findall(text)}
    times = {key: float(value) for key, value in TIME.findall(text)}
    lines = len(path.read_text().splitlines())
    verified = (
        record["status"] == "ok" and counts.get("E") == 0
        and counts.get("T") == counts.get("final_ops") == counts.get("validated_moves") == record["score"] == lines
        and lines <= 100000
    )
    structure = None
    if verified and verify_output:
        structure = replay(ROOT / "tools/in" / record["case_name"], path)
        assert structure["counts"]["T"] == lines
        assert counts["network_saved"] == counts["pre_network_ops"] - counts["post_network_ops"]
    return {**record, "counts": counts, "times_ms": times, "verified": verified, "structure": structure}


def summarize_cases(cases, baseline):
    result = summarize(cases, {BASELINE: baseline})
    good = [c for c in cases if c["verified"]]
    if len(good) != len(cases):
        return result
    sums = result["counts_sum"]
    before = sum(baseline[c["case_name"]]["counts"]["post_chain_ops"] for c in cases)
    baseline_final = sum(baseline[c["case_name"]]["score"] for c in cases)
    result["phase_comparison"] = {
        "before_network_minus_baseline_post_chain": sums["pre_network_ops"] - before,
        "network_saved": sums["network_saved"],
        "baseline_after_chain_saved": before - baseline_final,
        "after_network_saved": sums["post_network_ops"] - result["total_T"],
        "note": "Pre-phase differences include time-dependent search variation; late phases use different neighborhoods.",
    }
    result["group_counts"] = {
        "before": sums["pre_network_groups"],
        "after": sums["post_network_groups"],
        "increased_cases": sum(c["counts"]["post_network_groups"] > c["counts"]["pre_network_groups"] for c in cases),
        "decreased_cases": sum(c["counts"]["post_network_groups"] < c["counts"]["pre_network_groups"] for c in cases),
    }
    replays = [c["structure"]["counts"] for c in cases]
    result["output_structure"] = {
        key: sum(c.get(key, 0) for c in replays)
        for key in ("T", "W", "moved", "mixed_moves", "single_moves", "large_color_single_moves", "setup_ops")
    }
    result["mechanism_passed"] = all(sums.get(key, 0) > 0 for key in (
        "network_models", "network_support_models", "network_metric_calls",
        "network_proposals", "network_trials", "network_valid", "network_pinned_checks",
    ))
    result["improvement_activated"] = sums.get("network_accepted", 0) > 0 and sums.get("network_saved", 0) > 0
    return result


def main():
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, "b14_network_regroup")
    prior = latest_run(records, BASELINE, "pro_scaffold_chain_port")
    assert len(current) == len({c["case_name"] for c in current}) == 100
    assert {c["case_name"] for c in current} == {c["case_name"] for c in prior}
    baseline = {c["case_name"]: read_case(c, False) for c in prior}
    assert all(c["verified"] for c in baseline.values())
    cases = [read_case(c, True) for c in current]
    digest = hashlib.sha256((ROOT / "src/bin" / f"{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == EXPECTED_SHA256, "Solver changed after the preflight run"
    result = {
        "bin": BIN, "run_id": current[0]["run_id"], "baseline_run_id": prior[0]["run_id"],
        "solver_sha256": digest,
        "all_100": summarize_cases(cases, baseline),
        "generated_99": summarize_cases([c for c in cases if c["case_name"] != "0000.txt"], baseline),
        "cases": cases,
    }
    all_cases, generated = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == 100 and all_cases["cases_under_2000_ms"] == 100
        and generated.get("mechanism_passed", False) and generated.get("improvement_activated", False)
        and all_cases.get("comparisons", {}).get(BASELINE, {}).get("delta_T", 0) < 0
    )
    path = ROOT / "adhoc/v011_evaluation_summary.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(path.relative_to(ROOT)), "run_id": result["run_id"], "adopt": result["adopt"],
        "verified_cases": all_cases["verified_cases"], "mean_T": all_cases.get("mean_T"),
        "total_T": all_cases.get("total_T"), "comparisons": all_cases.get("comparisons"),
        "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "generated_99_phase": generated.get("phase_comparison"),
        "generated_99_mechanism": {
            key: {"total": value, "cases": generated["cases_positive"][key]}
            for key, value in generated["counts_sum"].items() if key.startswith("network_")
        },
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
