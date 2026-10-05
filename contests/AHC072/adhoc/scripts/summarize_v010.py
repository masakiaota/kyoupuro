#!/usr/bin/env python3
"""Compare the saved v010 port evaluation with v008/v009, without running solvers."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from statistics import mean


ROOT = Path(__file__).resolve().parents[2]
BIN = "v010_scaffold_chain"
LABEL = "pro_scaffold_chain_port"
BASELINES = {
    "v008_lightweight": "lightweight_all",
    "v009_relay_weave": "b12_relay_weave",
}
COUNT = re.compile(r"^\[summary\.count\] ([^=]+)=(-?\d+)$", re.MULTILINE)
TIME = re.compile(r"^\[summary\.time_ms\] ([^=]+)=([\d.]+)$", re.MULTILINE)


def latest_run(records: list[dict], bin_name: str, label: str) -> list[dict]:
    matching = [r for r in records if r["bin"] == bin_name and r["label"] == label]
    if not matching:
        raise ValueError(f"No recorded evaluation: {bin_name}, label={label}")
    return [r for r in matching if r["run_id"] == matching[-1]["run_id"]]


def summarize(cases: list[dict], baselines: dict[str, dict[str, dict]]) -> dict:
    successful = [c for c in cases if c["status"] == "ok"]
    count_keys = sorted({key for c in successful for key in c["counts"]})
    time_keys = sorted({key for c in successful for key in c["times_ms"]})
    summary = {
        "cases": len(cases),
        "successful": len(successful),
        "failed_cases": [c["case_name"] for c in cases if c["status"] != "ok"],
        "verified_cases": sum(c["verified"] for c in cases),
        "mean_elapsed_ms": mean(c["elapsed"] for c in cases),
        "max_elapsed_ms": max(c["elapsed"] for c in cases),
        "cases_under_2000_ms": sum(c["elapsed"] < 2000 for c in cases),
        "counts_sum": {key: sum(c["counts"].get(key, 0) for c in successful) for key in count_keys},
        "cases_positive": {key: sum(c["counts"].get(key, 0) > 0 for c in successful) for key in count_keys},
        "mean_times_ms": {
            key: mean(c["times_ms"].get(key, 0) for c in successful) for key in time_keys
        },
    }
    if len(successful) == len(cases) and all(c["verified"] for c in cases):
        total = sum(c["score"] for c in cases)
        summary["total_T"] = total
        summary["mean_T"] = total / len(cases)
        summary["comparisons"] = {}
        for name, baseline in baselines.items():
            before = sum(baseline[c["case_name"]]["score"] for c in cases)
            delta = [c["score"] - baseline[c["case_name"]]["score"] for c in cases]
            summary["comparisons"][name] = {
                "baseline_T": before,
                "baseline_mean_T": before / len(cases),
                "delta_T": total - before,
                "delta_percent": 100 * (total - before) / before,
                "wins": sum(d < 0 for d in delta),
                "ties": sum(d == 0 for d in delta),
                "losses": sum(d > 0 for d in delta),
            }
    return summary


def main() -> None:
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    baseline_runs = {name: latest_run(records, name, label) for name, label in BASELINES.items()}
    baselines = {name: {r["case_name"]: r for r in run} for name, run in baseline_runs.items()}
    assert len(current) == len({r["case_name"] for r in current}) == 100
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(r["status"] == "ok" for r in baseline.values())

    cases = []
    for record in current:
        path = ROOT / record["stdout_path"]
        text = path.with_name(path.name + ".err").read_text()
        counts = {key: int(value) for key, value in COUNT.findall(text)}
        times_ms = {key: float(value) for key, value in TIME.findall(text)}
        lines = len(path.read_text().splitlines())
        verified = (
            record["status"] == "ok"
            and counts.get("E") == 0
            and counts.get("T") == counts.get("final_ops") == counts.get("validated_moves") == record["score"] == lines
            and lines <= 100000
        )
        cases.append({**record, "counts": counts, "times_ms": times_ms, "verified": verified})

    result = {
        "bin": BIN,
        "run_id": current[0]["run_id"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in baseline_runs.items()},
        "solver_sha256": hashlib.sha256((ROOT / f"src/bin/{BIN}.cpp").read_bytes()).hexdigest(),
        "all_100": summarize(cases, baselines),
        "generated_99": summarize([c for c in cases if c["case_name"] != "0000.txt"], baselines),
    }
    if "mean_T" in result["all_100"]:
        result["mean_T_minus_expected_263"] = result["all_100"]["mean_T"] - 263
    output = ROOT / "adhoc/v010_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)),
        "run_id": result["run_id"],
        "verified_cases": result["all_100"]["verified_cases"],
        "total_T": result["all_100"].get("total_T"),
        "mean_T": result["all_100"].get("mean_T"),
        "comparisons": result["all_100"].get("comparisons"),
        "max_elapsed_ms": result["all_100"]["max_elapsed_ms"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
