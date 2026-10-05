#!/usr/bin/env python3
"""Verify saved event-driven LNS outputs and compare them without running solvers."""

import hashlib
import json
from pathlib import Path

from analyze_v010_structure import replay
from summarize_v010 import COUNT, TIME, latest_run, summarize


ROOT = Path(__file__).resolve().parents[2]
BIN = "v020_o3_event_lns"
LABEL = "o3_event_lns"
BASELINES = {
    "v019_temporal_lns": "temporal_identity_lns",
    "v017_timer_origin": "timer_origin_local",
}
ERROR_KEYS = ("baseline_recovery", "final_recovery", "construction_errors", "lns_errors")


def read_case(record):
    path = ROOT / record["stdout_path"]
    log_path = path.with_name(path.name + ".err")
    text = log_path.read_text() if log_path.exists() else ""
    counts = {key: int(value) for key, value in COUNT.findall(text)}
    times = {key: float(value) for key, value in TIME.findall(text)}
    lines = len(path.read_text().splitlines()) if path.exists() else -1
    verified = (
        record["status"] == "ok" and counts.get("E") == 0
        and counts.get("T") == counts.get("final_ops") == counts.get("validated_moves") == record["score"] == lines
        and 0 <= lines <= 100000
    )
    if verified:
        structure = replay(ROOT / "tools/in" / record["case_name"], path)
        assert structure["counts"]["T"] == lines
        assert structure["counts"]["M"] == counts["delivered_slimes"]
        assert counts["lns_saved"] == counts["pre_lns_ops"] - lines
        if not counts["final_recovery"]:
            assert counts["lns_saved"] == counts["lns_initial_shortcut_saved"] + counts["lns_initial_reorder_saved"] + counts["lns_best_saved"] >= 0
        assert 0 <= counts["lns_improvements"] <= counts["lns_accepted"] <= counts["lns_attempts"]
        assert counts["lns_best_saved"] == (
            counts["lns_regular_saved"] + counts["lns_packet_saved"] + counts["lns_flexible_saved"]
            + counts["lns_reorder_saved"] - counts["lns_initial_reorder_saved"]
        )
        assert 0 <= counts["lns_flexible_attempts"] <= counts["lns_block_attempts"] <= counts["lns_attempts"]
        assert 0 <= counts["lns_flexible_accepted"] <= counts["lns_block_accepted"] <= counts["lns_accepted"]
        assert counts["lns_flexible_accepted"] <= counts["lns_flexible_attempts"]
        assert counts["lns_block_accepted"] <= counts["lns_block_attempts"]
        assert 0 <= counts["lns_flexible_improvements"] <= counts["lns_block_improvements"] <= counts["lns_improvements"]
        assert 0 <= counts["lns_uphill"] <= counts["lns_accepted"]
        assert 0 <= counts["event_reopens"] <= 2 * counts["event_layers"]
        assert counts["construction_attempts"] >= counts["construction_completed"]
        assert times["search_limit"] == 1544.0
    return {**record, "counts": counts, "times_ms": times, "verified": verified,
            "diagnostics": [line for line in text.splitlines() if "diagnostic:" in line]}


def compare(cases, baselines):
    result = summarize(cases, baselines)
    counts = result["counts_sum"]
    result["event_lns_mechanism_passed"] = all(counts.get(key, 0) > 0 for key in (
        "lns_attempts", "lns_insertions", "lns_accepted", "lns_improvements",
        "lns_block_attempts", "lns_flexible_attempts", "lns_reorder_attempts",
        "event_layers", "event_reopens",
    ))
    result["error_count"] = sum(counts.get(key, 0) for key in ERROR_KEYS)
    result["lns_improved_cases"] = sum(c["counts"].get("lns_saved", 0) > 0 for c in cases)
    result["construction_saved"] = sum(c["counts"].get("baseline_ops", 0) - c["counts"].get("pre_lns_ops", 0) for c in cases)
    return result


def main():
    audit = json.loads((ROOT / "adhoc/v020_port_audit/static_verification.json").read_text())
    digest = hashlib.sha256((ROOT / f"src/bin/{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == audit["solver_sha256"], "Solver changed after the pre-evaluation audit"
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = {name: latest_run(records, name, label) for name, label in BASELINES.items()}
    baselines = {name: {r["case_name"]: r for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(r["status"] == "ok" and r["local"] for r in baseline.values())
    cases = [read_case(record) for record in current]
    result = {
        "bin": BIN, "parents": ["v019"], "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in prior.items()},
        "original_source_sha256": audit["original_sha256"], "solver_sha256": digest,
        "all_100": compare(cases, baselines),
        "generated_99": compare([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": compare([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "cases": cases,
    }
    all_cases, generated = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == 0 and generated["event_lns_mechanism_passed"]
        and all_cases.get("comparisons", {}).get("v019_temporal_lns", {}).get("delta_T", 0) < 0
    )
    output = ROOT / "adhoc/v020_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)), "run_id": result["run_id"],
        "adopt": result["adopt"], "verified_cases": all_cases["verified_cases"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"),
        "mean_elapsed_ms": all_cases["mean_elapsed_ms"], "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "error_count": all_cases["error_count"],
        "generated_99_mechanism_passed": generated["event_lns_mechanism_passed"],
        "generated_99_lns": {
            key: {"total": value, "cases": generated["cases_positive"][key]}
            for key, value in generated["counts_sum"].items() if key.startswith(("lns_", "event_"))
        },
        "generated_99_construction_saved": generated["construction_saved"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
