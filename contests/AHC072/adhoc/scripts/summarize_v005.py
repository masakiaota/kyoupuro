#!/usr/bin/env python3
"""v005の保存済み評価・機構ログを集計する。solverは実行しない。"""

import json
from pathlib import Path
import re
from statistics import mean


ROOT = Path(__file__).resolve().parents[2]
BIN = "v005_mobile_spring"
BASELINE_RUN = "20260925T232056+0900_v003_spring_network_356717"


def summarize(rows):
    if not rows:
        return {"cases": 0}
    counts = [row["counts"] for row in rows]
    current = sum(row["score"] for row in rows)
    baseline = sum(row["baseline_score"] for row in rows)
    deltas = [row["score"] - row["baseline_score"] for row in rows]
    result = {
        "cases": len(rows),
        "score_sum": current,
        "score_mean": current / len(rows),
        "score_min": min(row["score"] for row in rows),
        "score_max": max(row["score"] for row in rows),
        "baseline_sum": baseline,
        "delta": current - baseline,
        "reduction_percent": 100 * (baseline - current) / baseline,
        "wins": sum(delta < 0 for delta in deltas),
        "ties": sum(delta == 0 for delta in deltas),
        "losses": sum(delta > 0 for delta in deltas),
        "avg_elapsed": mean(row["elapsed"] for row in rows),
        "max_elapsed": max(row["elapsed"] for row in rows),
        "cases_with_relocations": sum(c["relocations"] > 0 for c in counts),
        "cases_improved_within_run": sum(c["relocation_gain"] > 0 for c in counts),
        "cases_without_eligible_pads": sum(c["eligible_mobile_pads"] == 0 for c in counts),
    }
    for key in ["before_relocation_T", "final_T", "relocation_gain", "relocations", "relocated_mass",
                "relocation_moves", "two_site_used_pads", "original_support_long_moves",
                "relocated_support_long_moves", "eligible_mobile_pads", "relocation_proposals",
                "relocation_evaluated", "relocation_accepted", "relocation_best_updates",
                "relocation_changed_phases", "relocation_reused_phases", "relocation_two_site_rejected",
                "setup_moves", "built_pads"]:
        result[key] = sum(c[key] for c in counts)
    for key in ["anneal", "relocation_anneal", "restore_and_validate", "total"]:
        result["avg_" + key + "_ms"] = mean(row["times_ms"][key] for row in rows)
    return result


def main():
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    run_id = next(row["run_id"] for row in reversed(records) if row["bin"] == BIN)
    selected = [row for row in records if row["run_id"] == run_id]
    baseline = {row["case_name"]: row["score"] for row in records if row["run_id"] == BASELINE_RUN}
    failures = [row for row in selected if row["status"] != "ok"]
    rows = []
    for record in selected:
        if record["status"] != "ok":
            continue
        output = ROOT / record["stdout_path"]
        text = output.with_suffix(".txt.err").read_text()
        counts = {key: int(value) for key, value in re.findall(r"\[summary.count\] (\w+)=(\d+)", text)}
        times = {key: float(value) for key, value in re.findall(r"\[summary.time_ms\] (\w+)=([\d.]+)", text)}
        assert "[summary] fallback_count=0" in text
        assert counts["remaining_slimes"] == 0
        assert counts["final_T"] == record["score"] == len(output.read_text().splitlines())
        assert counts["before_relocation_T"] - counts["final_T"] == counts["relocation_gain"]
        assert counts["two_site_used_pads"] == counts["relocations"]
        rows.append({**record, "baseline_score": baseline[record["case_name"]], "counts": counts, "times_ms": times})
    generated = [row for row in rows if row["case_name"] != "0000.txt"]
    result = {
        "run_id": run_id,
        "baseline_run_id": BASELINE_RUN,
        "recorded_cases": len(selected),
        "failures": failures,
        "all_100": summarize(rows),
        "generated_99": summarize(generated),
        "with_relocations": summarize([row for row in generated if row["counts"]["relocations"]]),
        "without_eligible_pads": summarize([row for row in generated if not row["counts"]["eligible_mobile_pads"]]),
        "cases": rows,
    }
    path = ROOT / "adhoc/v005_evaluation_summary.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key != "cases"}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
