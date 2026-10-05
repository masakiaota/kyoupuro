#!/usr/bin/env python3
"""v004の保存済み評価とTraceStatsを集計する。solverは実行しない。"""

import json
from pathlib import Path
import re
from statistics import mean, median


ROOT = Path(__file__).resolve().parents[2]
BIN = "v004_mixed_spring"
BASELINE_RUN = "20260925T232056+0900_v003_spring_network_356717"


def load_trace(path):
    text = path.read_text()
    counts = {key: int(value) for key, value in re.findall(r"\[summary.count\] (\w+)=(\d+)", text)}
    times = {key: float(value) for key, value in re.findall(r"\[summary.time_ms\] (\w+)=([\d.]+)", text)}
    counts["fallback_count"] = int(re.search(r"\[summary\] fallback_count=(\d+)", text).group(1))
    return counts, times


def summarize(rows):
    result = {"cases": len(rows)}
    if not rows:
        return result
    current = sum(row["score"] for row in rows)
    baseline = sum(row["baseline_score"] for row in rows)
    before = sum(row["trace"]["before_mixing_T"] for row in rows)
    result.update({
        "T_sum": current,
        "T_avg": current / len(rows),
        "baseline_sum": baseline,
        "delta": current - baseline,
        "reduction_percent": (baseline - current) * 100 / baseline,
        "wins": sum(row["score"] < row["baseline_score"] for row in rows),
        "ties": sum(row["score"] == row["baseline_score"] for row in rows),
        "losses": sum(row["score"] > row["baseline_score"] for row in rows),
        "elapsed_avg": mean(row["elapsed"] for row in rows),
        "elapsed_max": max(row["elapsed"] for row in rows),
        "before_mixing_T_sum": before,
        "mixed_gain": before - current,
        "mixed_gain_percent": (before - current) * 100 / before,
        "mixed_improved_cases": sum(row["score"] < row["trace"]["before_mixing_T"] for row in rows),
    })
    keys = ["remaining_slimes", "fallback_count", "mixed_shipments", "mixed_slimes", "mixed_shipment_moves",
            "mixed_carry_moves", "mixed_long_jump_moves", "source_returned", "home_launch_long_moves",
            "mixed_proposals", "mixed_evaluated", "mixed_accepted", "mixed_routes_searched",
            "mixed_route_cache_hits", "mixed_states_popped", "mixed_unreachable", "built_pads", "setup_moves"]
    result["counts_sum"] = {key: sum(row["trace"][key] for row in rows) for key in keys}
    result["cases_positive"] = {key: sum(row["trace"][key] > 0 for row in rows) for key in keys[:9]}
    result["mixed_evaluated_median"] = median(row["trace"]["mixed_evaluated"] for row in rows)
    result["time_ms_avg"] = {key: mean(row["times"][key] for row in rows) for key in rows[0]["times"]}
    return result


def main():
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    run = next(row["run_id"] for row in reversed(records) if row["bin"] == BIN)
    baseline = {row["case_name"]: row for row in records if row["run_id"] == BASELINE_RUN}
    current = [row for row in records if row["run_id"] == run]
    assert len(current) == 100 and all(row["status"] == "ok" for row in current)
    rows = []
    for record in current:
        row = dict(record)
        output_path = ROOT / row["stdout_path"]
        counts, times = load_trace(output_path.with_suffix(".txt.err"))
        assert counts["remaining_slimes"] == 0
        assert row["score"] == counts["final_T"] == len(output_path.read_text().splitlines())
        lines = (ROOT / "tools/in" / row["case_name"]).read_text().splitlines()
        N, K = map(int, lines[0].split())
        C = "".join(lines[1:])
        M = sum("a" <= char <= "l" for char in C)
        density = M / (N * N - C.count("#") - K)
        row.update({"baseline_score": baseline[row["case_name"]]["score"], "trace": counts, "times": times,
                    "N": N, "K": K, "M": M, "density": density})
        rows.append(row)
    generated = [row for row in rows if row["case_name"] != "0000.txt"]
    groups = {
        "all_100": rows,
        "generated_99": generated,
        "small_M_le_35": [row for row in generated if row["M"] <= 35],
        "large_M_ge_100": [row for row in generated if row["M"] >= 100],
        "many_colors_K_ge_10": [row for row in generated if row["K"] >= 10],
        "sparse_lt_25_percent": [row for row in generated if row["density"] < .25],
        "dense_ge_50_percent": [row for row in generated if row["density"] >= .50],
    }
    summaries = {name: summarize(group) for name, group in groups.items()}
    result = {"run_id": run, "baseline_run_id": BASELINE_RUN, "summaries": summaries, "cases": rows}
    (ROOT / "adhoc/v004_analysis.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"run_id": run, "summaries": summaries}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
