#!/usr/bin/env python3
"""Summarize the saved B-12 evaluation; never execute or change a solver."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from statistics import mean


ROOT = Path(__file__).resolve().parents[2]
BIN = "v009_relay_weave"
BASELINE = "v008_lightweight"
OUTPUT = ROOT / "adhoc/v009_evaluation_summary.json"
COUNT = re.compile(r"^\[summary\.count\] ([^=]+)=(-?\d+)$", re.MULTILINE)
TIME = re.compile(r"^\[summary\.time_ms\] ([^=]+)=([\d.]+)$", re.MULTILINE)


def latest_run(records: list[dict], bin_name: str, label: str) -> list[dict]:
    matching = [r for r in records if r["bin"] == bin_name and r["label"] == label]
    if not matching:
        raise ValueError(f"No recorded evaluation: {bin_name}, label={label}")
    run_id = matching[-1]["run_id"]
    return [r for r in matching if r["run_id"] == run_id]


def read_trace(path: Path) -> dict:
    text = path.read_text()
    return {
        "counts": {key: int(value) for key, value in COUNT.findall(text)},
        "times_ms": {key: float(value) for key, value in TIME.findall(text)},
        "fallback_zero": "[summary] fallback_count=0" in text,
    }


def summarize_group(cases: list[dict], baseline: dict[str, dict]) -> dict:
    successful = [c for c in cases if c["status"] == "ok"]
    keys = sorted({key for c in successful for key in c["trace"]["counts"]})
    time_keys = sorted({key for c in successful for key in c["trace"]["times_ms"]})
    summary = {
        "cases": len(cases),
        "successful": len(successful),
        "failed_cases": [c["case_name"] for c in cases if c["status"] != "ok"],
        "verified_cases": sum(c["verified"] for c in cases),
        "mean_elapsed_ms": mean(c["elapsed"] for c in cases),
        "max_elapsed_ms": max(c["elapsed"] for c in cases),
        "counts_sum": {
            key: sum(c["trace"]["counts"].get(key, 0) for c in successful) for key in keys
        },
        "cases_positive": {
            key: sum(c["trace"]["counts"].get(key, 0) > 0 for c in successful) for key in keys
        },
        "mean_times_ms": {
            key: mean(c["trace"]["times_ms"].get(key, 0) for c in successful) for key in time_keys
        },
    }
    if successful and len(successful) == len(cases) and all(c["verified"] for c in cases):
        before = sum(baseline[c["case_name"]]["score"] for c in cases)
        after = sum(c["score"] for c in cases)
        initial = sum(c["trace"]["counts"]["initial_T"] for c in cases)
        delta = [c["score"] - baseline[c["case_name"]]["score"] for c in cases]
        summary["comparison"] = {
            "baseline_T": before,
            "T": after,
            "delta_T": after - before,
            "delta_percent": 100 * (after - before) / before,
            "wins": sum(d < 0 for d in delta),
            "ties": sum(d == 0 for d in delta),
            "losses": sum(d > 0 for d in delta),
            "initial_T": initial,
            "search_gain": initial - after,
            "search_gain_percent": 100 * (initial - after) / initial,
        }
    return summary


def main() -> None:
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, "b12_relay_weave")
    before = latest_run(records, BASELINE, "lightweight_all")
    baseline = {r["case_name"]: r for r in before}
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert {r["case_name"] for r in current} == set(baseline)
    assert all(r["status"] == "ok" for r in before)

    cases = []
    for record in current:
        output_path = ROOT / record["stdout_path"]
        trace = read_trace(output_path.with_name(output_path.name + ".err"))
        lines = len(output_path.read_text().splitlines())
        verified = (
            record["status"] == "ok"
            and trace["counts"].get("E") == 0
            and trace["counts"].get("T") == record["score"] == lines
            and lines <= 100000
            and trace["fallback_zero"]
        )
        cases.append({**record, "trace": trace, "verified": verified})

    summary = {
        "bin": BIN,
        "run_id": current[0]["run_id"],
        "baseline": BASELINE,
        "baseline_run_id": before[0]["run_id"],
        "solver_sha256": hashlib.sha256((ROOT / f"src/bin/{BIN}.cpp").read_bytes()).hexdigest(),
        "all_100": summarize_group(cases, baseline),
        "generated_99": summarize_group([c for c in cases if c["case_name"] != "0000.txt"], baseline),
    }
    OUTPUT.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(OUTPUT.relative_to(ROOT)),
        "run_id": summary["run_id"],
        "verified_cases": summary["all_100"]["verified_cases"],
        "comparison": summary["all_100"].get("comparison"),
        "max_elapsed_ms": summary["all_100"]["max_elapsed_ms"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
