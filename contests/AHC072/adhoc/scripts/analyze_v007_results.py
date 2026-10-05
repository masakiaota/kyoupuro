#!/usr/bin/env python3
"""v007の保存済みevalをv006と比較し、出力と差分評価の実測を確認する。"""

import hashlib
import json
from pathlib import Path
from statistics import mean, median

from analyze_approach_outputs import replay, trace
from analyze_v006_results import latest_records


ROOT = Path(__file__).resolve().parents[2]
PARENT = "v006_branch_reconnect"
SOLVER = "v007_incremental"
SOURCE_SHA256 = "54a5628e3d01387d461e4eac8d4270f1dc01006d1746e55ae9d98431102d03f6"
COUNT_KEYS = [
    "sa_iterations", "sa_evaluated", "phase_builds", "phase_simulations", "full_simulations",
    "partial_simulations", "partial_nodes", "partial_full_equivalent_nodes", "flow_delta_nodes",
    "retagged_nodes", "unchanged_inventory_hits", "priority_order_reuses", "schedule_without_simulation",
    "reference_checks", "inventory_reference_checks", "donor_calls", "donor_scanned",
    "donor_full_scan_equivalent", "layout_changed_cells", "refit_changed_phases",
    "dp_initialized_cells", "dp_bulk_clear_cells_avoided", "order_bitset_builds",
    "reconnect_dp_calls", "reconnect_multi_edge_accepted", "reconnect_best_updates",
    "final_one_flow_edges", "final_packing_gap", "built_pads", "setup_moves",
]
COMMON_KEYS = ["sa_iterations", "sa_evaluated", "phase_builds", "phase_simulations", "reconnect_dp_calls"]


def summarize(rows):
    before = sum(row["parent_T"] for row in rows)
    after = sum(row["T"] for row in rows)
    counts = {key: sum(row["trace"][key] for row in rows) for key in COUNT_KEYS}
    parent_counts = {key: sum(row["parent_trace"][key] for row in rows) for key in COMMON_KEYS}
    result = {
        "cases": len(rows),
        "parent_total": before,
        "total": after,
        "mean": after / len(rows),
        "delta": after - before,
        "reduction_percent": 100 * (before - after) / before,
        "wins": sum(row["T"] < row["parent_T"] for row in rows),
        "ties": sum(row["T"] == row["parent_T"] for row in rows),
        "losses": sum(row["T"] > row["parent_T"] for row in rows),
        "elapsed_mean_ms": mean(row["elapsed"] for row in rows),
        "elapsed_max_ms": max(row["elapsed"] for row in rows),
        "all_E_zero": all(row["trace"]["remaining_slimes"] == 0 for row in rows),
        "counts": counts,
        "positive_cases": {key: sum(row["trace"][key] > 0 for row in rows) for key in COUNT_KEYS},
        "parent_counts": parent_counts,
        "common_count_ratios": {key: counts[key] / parent_counts[key] for key in COMMON_KEYS},
        "evaluated_ratio_case_median": median(row["trace"]["sa_evaluated"] / row["parent_trace"]["sa_evaluated"] for row in rows),
        "partial_node_reduction_percent": 100 * (1 - counts["partial_nodes"] / counts["partial_full_equivalent_nodes"]),
        "donor_scan_reduction_percent": 100 * (1 - counts["donor_scanned"] / counts["donor_full_scan_equivalent"]),
        "reconnect_stage_reduction": sum(row["trace"]["before_reconnect_T"] - row["T"] for row in rows),
        "time_ms_mean": {key: mean(row["trace"][key] for row in rows) for key in ["precompute", "initialize", "anneal", "reconnect", "restore_and_validate", "total"]},
        "parent_single_delivery_moves": sum(row["parent_replay"]["delivery_single_moves"] for row in rows),
        "single_delivery_moves": sum(row["replay"]["delivery_single_moves"] for row in rows),
    }
    for key, duration in [("sa_evaluated", "total"), ("reconnect_dp_calls", "reconnect")]:
        seconds = sum(row["trace"][duration] for row in rows) / 1000
        parent_seconds = sum(row["parent_trace"][duration] for row in rows) / 1000
        result[key + "_per_second"] = counts[key] / seconds
        result["parent_" + key + "_per_second"] = parent_counts[key] / parent_seconds
    return result


def main():
    parent = latest_records(PARENT)
    current = latest_records(SOLVER)
    assert len(parent) == len(current) == 100
    assert all(row["status"] == "ok" for row in current.values())
    source_hash = hashlib.sha256((ROOT / "src/bin" / (SOLVER + ".cpp")).read_bytes()).hexdigest()
    assert source_hash == SOURCE_SHA256
    prior = json.loads((ROOT / "adhoc/v006_analysis.json").read_text())
    assert prior["run_id"] == next(iter(parent.values()))["run_id"]
    prior_cases = {row["case"]: row for row in prior["cases"]}
    cases = []
    for name, record in sorted(current.items()):
        path = ROOT / record["stdout_path"]
        log_path = path.with_suffix(".txt.err")
        counts = trace(log_path)
        assert "[summary] fallback_count=0" in log_path.read_text()
        assert counts["final_T"] == record["score"]
        assert counts["remaining_slimes"] == 0 and counts["reference_checks"] > 0
        assert counts["before_reconnect_T"] >= counts["final_T"]
        old = prior_cases[name]
        assert old["T"] == parent[name]["score"]
        cases.append({
            "case": name,
            "T": record["score"],
            "parent_T": parent[name]["score"],
            "elapsed": record["elapsed"],
            "trace": counts,
            "parent_trace": old["trace"],
            "input": old["input"],
            "replay": replay(ROOT / "tools/in" / name, path, counts, "C"),
            "parent_replay": old["replay"],
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
    required = ["partial_simulations", "unchanged_inventory_hits", "priority_order_reuses", "reference_checks"]
    checks = {
        "all_success_E_zero": summaries["all_100"]["all_E_zero"],
        "all_under_2000_ms": summaries["all_100"]["elapsed_max_ms"] < 2000,
        "mechanisms_active": all(summaries["generated_99"]["counts"][key] > 0 for key in required),
        "fewer_partial_nodes": summaries["generated_99"]["partial_node_reduction_percent"] > 0,
        "better_total_T": summaries["all_100"]["delta"] < 0,
    }
    result = {
        "run_id": next(iter(current.values()))["run_id"],
        "parent_run_id": next(iter(parent.values()))["run_id"],
        "source_sha256": source_hash,
        "adoption_checks": checks,
        "adopted": all(checks.values()),
        "summaries": summaries,
        "cases": cases,
    }
    (ROOT / "adhoc/v007_analysis.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key != "cases"}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
