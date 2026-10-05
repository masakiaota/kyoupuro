#!/usr/bin/env python3
"""保存済み教師の最初の試行と後続探索を集計する。solverと学習は実行しない。"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

import numpy as np


def analyze(run, output):
    items = [json.loads(line) for line in (run / "inputs.jsonl").read_text().splitlines()]
    # 全期間へ等間隔に分散した1,024ブロックから、調整用の両匹数群を1入力ずつ選ぶ。
    # 結果を読む前に入力番号だけで決め、学習済み重みの選択には使わない。
    chosen = [item for item in items if item["index"] // 8 % 8 == 0 and item["role"] == "validation"]
    stats = Counter()
    outcome = {}
    elapsed = []
    per_group = []
    def add(rows, name):
        if len(rows) < 2:
            return
        first = np.array([r["first_best_T"] for r in rows])
        final = np.array([r["best_T"] for r in rows])
        differing = final[:, None] < final[None, :]
        count = int(differing.sum())
        stats[name + "_groups"] += 1
        stats[name + "_final_varying_groups"] += int(final.min() != final.max())
        stats[name + "_first_varying_groups"] += int(first.min() != first.max())
        stats[name + "_final_differing_pairs"] += count
        stats[name + "_first_tied_final_differing_pairs"] += int((differing & (first[:, None] == first[None, :])).sum())
        stats[name + "_first_agree_final_differing_pairs"] += int((differing & (first[:, None] < first[None, :])).sum())
    for item in chosen:
        folder = run / "cases" / f"{item['index']:06d}"
        rows = [json.loads(line) for line in (folder / "candidates.jsonl").read_text().splitlines()]
        for r in rows:
            initial_current, initial_best = r["features"][5:7]
            first_gain = initial_best - r["first_best_T"]
            final_gain = initial_best - r["best_T"]
            key = r["first_outcome"]
            if key == "accepted":
                key += "_shorter" if r["first_T"] < initial_current else "_equal" if r["first_T"] == initial_current else "_uphill"
            bucket = outcome.setdefault(key, Counter())
            for b in (stats, bucket):
                b["rows"] += 1
                b["first_current_improved"] += int(r["first_T"] < initial_current)
                b["first_best_improved"] += int(first_gain > 0)
                b["final_best_improved"] += int(final_gain > 0)
                b["final_improved_without_first_best_improvement"] += int(final_gain > 0 and first_gain == 0)
                b["first_best_gain_sum"] += first_gain
                b["final_best_gain_sum"] += final_gain
            elapsed.append(r["first_elapsed"])
        for phase in range(4):
            group = sorted((r for r in rows if r["phase"] == phase), key=lambda r: r["rank"])
            add(group, "all")
            unchanged = [r for r in group if r["first_outcome"] in ("extract_failed", "insert_failed", "duplicate", "rejected")]
            add(unchanged, "first_solution_unchanged")
            if group:
                baseline = group[0]
                per_group.append({"index": item["index"], "phase": phase, "M": item["M"],
                                  "baseline_first_gain": baseline["features"][6] - baseline["first_best_T"],
                                  "baseline_final_gain": baseline["features"][6] - baseline["best_T"],
                                  "best_first_gain": baseline["features"][6] - min(r["first_best_T"] for r in group),
                                  "best_final_gain": baseline["features"][6] - min(r["best_T"] for r in group)})
    total_pairs = stats["all_final_differing_pairs"]
    result = {
        "source_run": str(run), "sample_rule": "index//8 % 8 == 0 and role == validation",
        "inputs": len(chosen), "M_ge_80": sum(item["M"] >= 80 for item in chosen),
        "stats": dict(stats), "outcomes": {k: dict(v) for k, v in outcome.items()},
        "first_elapsed_ms": {"mean": float(np.mean(elapsed) * 1000), "median": float(np.median(elapsed) * 1000),
                             "p95": float(np.percentile(elapsed, 95) * 1000), "max": float(np.max(elapsed) * 1000)},
        "first_tied_among_final_differing_pairs": stats["all_first_tied_final_differing_pairs"] / total_pairs,
        "first_gain_fraction_of_final_gain": stats["first_best_gain_sum"] / stats["final_best_gain_sum"],
        "observed_oracle_gaps": {
            "first": float(np.mean([g["best_first_gain"]-g["baseline_first_gain"] for g in per_group])),
            "final": float(np.mean([g["best_final_gain"]-g["baseline_final_gain"] for g in per_group])),
        },
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    (output / "sample.json").write_text(json.dumps(chosen, ensure_ascii=False, indent=2) + "\n")
    (output / "groups.jsonl").write_text("".join(json.dumps(g) + "\n" for g in per_group))
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    analyze(args.run.resolve(), args.output.resolve())
