#!/usr/bin/env python3
"""v006の保存済みevalをv003と比較する。solverの実行や解の生成は行わない。"""

import hashlib
import json
from pathlib import Path
from statistics import mean

from analyze_approach_outputs import replay, trace


ROOT = Path(__file__).resolve().parents[2]
PARENT = "v003_spring_network"
SOLVER = "v006_branch_reconnect"


def latest_records(bin_name):
    marker = '"bin":"' + bin_name + '"'
    rows = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines() if marker in line]
    run_id = rows[-1]["run_id"]
    return {row["case_name"]: row for row in rows if row["run_id"] == run_id}


def summarize(rows):
    before = sum(row["parent_T"] for row in rows)
    after = sum(row["T"] for row in rows)
    reconnect_before = sum(row["trace"]["before_reconnect_T"] for row in rows)
    keys = [
        "reconnect_dp_calls", "reconnect_terminals", "reconnect_changed_edges", "reconnect_accepted",
        "reconnect_multi_edge_accepted", "reconnect_improved", "reconnect_best_updates", "reconnect_gain_sum",
        "final_reconnected_phases", "final_reconnect_events", "final_one_flow_edges", "final_packing_gap",
        "built_pads", "setup_moves", "long_jump_moves", "sa_iterations", "sa_evaluated",
    ]
    return {
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
        "reconnect_stage_reduction": reconnect_before - after,
        "reconnect_stage_reduction_percent": 100 * (reconnect_before - after) / reconnect_before,
        "reconnect_stage_improved_cases": sum(row["trace"]["before_reconnect_T"] > row["T"] for row in rows),
        "counts": {key: sum(row["trace"][key] for row in rows) for key in keys},
        "positive_cases": {key: sum(row["trace"][key] > 0 for row in rows) for key in keys},
        "time_ms_mean": {key: mean(row["trace"][key] for row in rows) for key in ["anneal", "reconnect", "restore_and_validate", "total"]},
        "parent_single_delivery_moves": sum(row["parent_replay"]["delivery_single_moves"] for row in rows),
        "single_delivery_moves": sum(row["replay"]["delivery_single_moves"] for row in rows),
        "parent_delivery_moves": sum(row["parent_replay"]["delivery_moves"] for row in rows),
        "delivery_moves": sum(row["replay"]["delivery_moves"] for row in rows),
        "parent_work": sum(row["parent_replay"]["work"] for row in rows),
        "work": sum(row["replay"]["work"] for row in rows),
    }


def main():
    parent = latest_records(PARENT)
    current = latest_records(SOLVER)
    assert len(parent) == len(current) == 100
    assert all(row["status"] == "ok" for row in current.values())
    prior_analysis = json.loads((ROOT / "adhoc/approach_output_analysis_20260925.json").read_text())
    prior_cases = {row["case"]: row for row in prior_analysis["cases"]}
    cases = []
    for name, record in sorted(current.items()):
        path = ROOT / record["stdout_path"]
        log_path = path.with_suffix(".txt.err")
        counts = trace(log_path)
        assert "[summary] fallback_count=0" in log_path.read_text()
        assert counts["final_T"] == record["score"]
        assert counts["before_reconnect_T"] >= counts["final_T"]
        checked = replay(ROOT / "tools/in" / name, path, counts, "C")
        old = prior_cases[Path(name).stem]
        assert old["C"]["replay"]["T"] == parent[name]["score"]
        cases.append({
            "case": name,
            "T": record["score"],
            "parent_T": parent[name]["score"],
            "elapsed": record["elapsed"],
            "trace": counts,
            "input": old["input"],
            "replay": checked,
            "parent_replay": old["C"]["replay"],
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
    result = {
        "run_id": next(iter(current.values()))["run_id"],
        "parent_run_id": next(iter(parent.values()))["run_id"],
        "source_sha256": hashlib.sha256((ROOT / "src/bin" / (SOLVER + ".cpp")).read_bytes()).hexdigest(),
        "summaries": summaries,
        "cases": cases,
    }
    path = ROOT / "adhoc/v006_analysis.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"run_id": result["run_id"], "summaries": summaries}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
