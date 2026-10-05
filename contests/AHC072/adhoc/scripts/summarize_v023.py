#!/usr/bin/env python3
"""Summarize the preregistered four-way comparison and replay saved v023 outputs."""
import csv
import hashlib
import json
import statistics
from collections import defaultdict
from pathlib import Path

from summarize_v021 import compare, read_case
from summarize_v010 import latest_run

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "adhoc/v023_trial_undo"
BIN = "v023_lazy_trial_undo"


def main():
    checks = json.loads((ARTIFACTS / "checks.json").read_text())
    samples = list(csv.DictReader((ARTIFACTS / "samples.csv").open()))
    if len(samples) != 99 * 8 * 4:
        raise RuntimeError("Incomplete benchmark")
    groups = defaultdict(lambda: defaultdict(float))
    for row in samples:
        for key in ("cpu_ms", "wall_ms"):
            groups[(row["mode"], key)][int(row["round"])] += float(row[key])
    timings = {}
    for mode in ("A_copy", "A_undo", "B_copy", "B_undo"):
        timings[mode] = {}
        for key in ("cpu_ms", "wall_ms"):
            values = [groups[mode, key][i] for i in range(8)]
            timings[mode][key] = {"rounds": values, "median": statistics.median(values)}
    base = timings["A_copy"]["cpu_ms"]["median"]
    for mode, values in timings.items():
        values["relative_to_A_copy"] = values["cpu_ms"]["median"] / base
    b_undo = timings["B_undo"]["cpu_ms"]["median"]
    perf_pass = all(b_undo <= timings[m]["cpu_ms"]["median"] * 0.95 for m in ("A_copy", "A_undo"))
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, "lazy_trial_undo")
    prior = latest_run(records, "v021_paired_event_lns", "paired_event_lns")
    if len(current) != 100 or len({r["run_id"] for r in records if r["bin"] == BIN}) != 1:
        raise RuntimeError("Expected exactly one 100-case evaluation")
    cases = [read_case(r) for r in current]
    baselines = {"v021_paired_event_lns": {r["case_name"]: r for r in prior}}
    total = compare(cases, baselines)
    generated = compare([c for c in cases if c["case_name"] != "0000.txt"], baselines)
    mechanism = all(total["counts_sum"].get(k, 0) > 0 for k in (
        "trial_board_conversions", "trial_undo_attempts", "trial_undo_moves"))
    score_pass = (
        total["verified_cases"] == total["cases_under_2000_ms"] == 100
        and total["error_count"] == 0 and mechanism
        and total["comparisons"]["v021_paired_event_lns"]["delta_T"] <= 0
    )
    result = {
        "bin": BIN, "run_id": current[0]["run_id"], "baseline_run_id": prior[0]["run_id"],
        "solver_sha256": hashlib.sha256((ROOT / f"src/bin/{BIN}.cpp").read_bytes()).hexdigest(),
        "checks": checks, "fixed_work_timings": timings,
        "fixed_work_pass": perf_pass, "evaluation_pass": score_pass, "adopt": perf_pass and score_pass,
        "all_100": total, "generated_99": generated,
        "case0000": compare([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "cases": cases,
    }
    output = ARTIFACTS / "summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output), "adopt": result["adopt"], "fixed_work_pass": perf_pass,
        "evaluation_pass": score_pass, "checks": checks,
        "timings": {m: {"cpu_ms": t["cpu_ms"]["median"], "relative_to_A_copy": t["relative_to_A_copy"]}
                    for m, t in timings.items()},
        "verified_cases": total["verified_cases"], "max_elapsed_ms": total["max_elapsed_ms"],
        "error_count": total["error_count"], "comparisons": total["comparisons"],
        "trial_counts": {k: v for k, v in total["counts_sum"].items() if k.startswith("trial_")},
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
