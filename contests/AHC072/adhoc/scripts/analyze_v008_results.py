#!/usr/bin/env python3
"""保存済みv008出力を再生し、固定仕事量の検証とv006比をまとめる。"""

import hashlib
import json
from pathlib import Path
from statistics import mean

from analyze_approach_outputs import replay, trace
from analyze_v006_results import latest_records


ROOT = Path(__file__).resolve().parents[2]
SOLVER = "v008_lightweight"
PARENT_RUN = "20260926T001205+0900_v006_branch_reconnect_d22a2b"
SOURCE_HASH = "0d39e7d7d0040e322b55d536a61bec5af2d9bba16e5678bcfe746e932bc95dee"
KEYS = ["sa_evaluated", "sa_iterations", "phase_builds", "phase_simulations", "reconnect_dp_calls",
        "reconnect_multi_edge_accepted", "reconnect_best_updates", "built_pads", "setup_moves",
        "reconnect_workspace_reuses", "donor_workspace_reuses", "reconnect_workspace_extra_bytes",
        "donor_workspace_extra_bytes"]
COMMON_KEYS = ["sa_evaluated", "sa_iterations", "phase_builds", "phase_simulations", "reconnect_dp_calls"]


def summarize(rows):
    total = sum(row["T"] for row in rows)
    parent = sum(row["parent_T"] for row in rows)
    return {
        "cases": len(rows),
        "total": total,
        "parent_total": parent,
        "mean": total / len(rows),
        "delta": total - parent,
        "reduction_percent": 100 * (parent - total) / parent,
        "wins": sum(row["T"] < row["parent_T"] for row in rows),
        "ties": sum(row["T"] == row["parent_T"] for row in rows),
        "losses": sum(row["T"] > row["parent_T"] for row in rows),
        "elapsed_mean_ms": mean(row["elapsed"] for row in rows),
        "elapsed_max_ms": max(row["elapsed"] for row in rows),
        "all_E_zero": all(row["trace"]["remaining_slimes"] == 0 for row in rows),
        "counts": {key: sum(row["trace"][key] for row in rows) for key in KEYS},
        "positive_cases": {key: sum(row["trace"][key] > 0 for row in rows) for key in KEYS},
        "parent_counts": {key: sum(row["parent_trace"][key] for row in rows) for key in COMMON_KEYS},
        "historical_count_ratios": {key: sum(row["trace"][key] for row in rows) / sum(row["parent_trace"][key] for row in rows) for key in COMMON_KEYS},
        "reconnect_stage_reduction": sum(row["trace"]["before_reconnect_T"] - row["T"] for row in rows),
        "time_ms_mean": {key: mean(row["trace"][key] for row in rows) for key in ["precompute", "initialize", "anneal", "reconnect", "restore_and_validate", "total"]},
    }


def main():
    current = latest_records(SOLVER)
    assert len(current) == 100 and all(row["status"] == "ok" for row in current.values())
    parent = json.loads((ROOT / "adhoc/v006_analysis.json").read_text())
    assert parent["run_id"] == PARENT_RUN
    prior = {row["case"]: row for row in parent["cases"]}
    fixed = json.loads((ROOT / "adhoc/v008_fixed_work/summary.json").read_text())
    source_hash = hashlib.sha256((ROOT / "src/bin" / (SOLVER + ".cpp")).read_bytes()).hexdigest()
    assert source_hash == SOURCE_HASH == fixed["source_sha256"]
    assert all(row["checks_equal"] for row in fixed["cases"])
    cases = []
    for name, row in sorted(current.items()):
        output = ROOT / row["stdout_path"]
        log = output.with_suffix(".txt.err")
        counts = trace(log)
        assert "[summary] fallback_count=0" in log.read_text()
        assert counts["final_T"] == row["score"] and counts["remaining_slimes"] == 0
        assert counts["ceil_table_entries"] == 2005 and counts["ceil_table_bytes"] == 4010
        assert counts["adjacency_bytes_per_cell"] == 72 and counts["phase_bytes"] == 7240
        assert counts["reconnect_workspace_extra_bytes"] == counts["donor_workspace_extra_bytes"] == 0
        assert counts["reconnect_workspace_reuses"] > 0 and counts["donor_workspace_reuses"] > 0
        old = prior[name]
        cases.append({
            "case": name,
            "T": row["score"],
            "parent_T": old["T"],
            "elapsed": row["elapsed"],
            "trace": counts,
            "parent_trace": old["trace"],
            "input": old["input"],
            "replay": replay(ROOT / "tools/in" / name, output, counts, "C"),
        })
    generated = [row for row in cases if row["case"] != "0000.txt"]
    groups = {
        "all_100": cases,
        "generated_99": generated,
        "large_M_ge_100": [row for row in generated if row["input"]["M"] >= 100],
        "small_M_le_35": [row for row in generated if row["input"]["M"] <= 35],
        "many_colors_K_ge_10": [row for row in generated if row["input"]["K"] >= 10],
        "few_colors_K_le_6": [row for row in generated if row["input"]["K"] <= 6],
    }
    summaries = {key: summarize(rows) for key, rows in groups.items()}
    all_cases = summaries["all_100"]
    checks = {
        "all_success_E_zero": all_cases["all_E_zero"],
        "all_under_2000_ms": all_cases["elapsed_max_ms"] < 2000,
        "fixed_work_equal": all(row["checks_equal"] for row in fixed["cases"]),
        "no_workspace_growth": all_cases["counts"]["reconnect_workspace_extra_bytes"] == all_cases["counts"]["donor_workspace_extra_bytes"] == 0,
        "better_total_T": all_cases["delta"] < 0,
    }
    result = {
        "run_id": next(iter(current.values()))["run_id"],
        "parent_run_id": PARENT_RUN,
        "source_sha256": source_hash,
        "adoption_checks": checks,
        "adopted": all(checks.values()),
        "fixed_work": fixed,
        "summaries": summaries,
        "cases": cases,
    }
    (ROOT / "adhoc/v008_analysis.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key != "cases"}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
