#!/usr/bin/env python3
"""保存済みの塔A/B計測CSVを集計する。ベンチマークは再実行しない。"""

import csv
import json
import statistics
from pathlib import Path


def summarize(rows):
    variants = {}
    by_variant = {}
    for variant in ("A", "B"):
        samples = []
        for sample in range(8):
            selected = [r for r in rows if r["variant"] == variant and r["sample"] == sample]
            operations = sum(r["operations"] for r in selected)
            cpu_ns = sum(r["cpu_ns"] for r in selected)
            wall_ns = sum(r["wall_ns"] for r in selected)
            samples.append({"sample": sample, "cpu_ns_per_operation": cpu_ns / operations,
                            "wall_ns_per_operation": wall_ns / operations})
        by_variant[variant] = samples
        cpu_values = [r["cpu_ns_per_operation"] for r in samples]
        wall_values = [r["wall_ns_per_operation"] for r in samples]
        variants[variant] = {
            "median_cpu_ns_per_operation": statistics.median(cpu_values),
            "min_cpu_ns_per_operation": min(cpu_values),
            "max_cpu_ns_per_operation": max(cpu_values),
            "median_wall_ns_per_operation": statistics.median(wall_values),
            "samples": samples,
        }
    ratios = [b["cpu_ns_per_operation"] / a["cpu_ns_per_operation"]
              for a, b in zip(by_variant["A"], by_variant["B"])]
    ratio = variants["B"]["median_cpu_ns_per_operation"] / variants["A"]["median_cpu_ns_per_operation"]
    return {"variants": variants, "ratio_of_medians_B_over_A": ratio,
            "paired_ratios_B_over_A": ratios, "A_wins": sum(r > 1 for r in ratios),
            "B_wins": sum(r < 1 for r in ratios),
            "shorter": "A" if ratio > 1 else "B",
            "time_reduction_percent": (1 - 1 / ratio if ratio > 1 else 1 - ratio) * 100}


def main():
    root = Path(__file__).resolve().parents[2]
    directory = root / "adhoc/tower_ab_benchmark"
    with (directory / "samples.csv").open() as stream:
        rows = list(csv.DictReader(stream))
    for row in rows:
        for key in ("sample", "rounds", "operations", "checksum"):
            row[key] = int(row[key])
        for key in ("cpu_ns", "wall_ns"):
            row[key] = float(row[key])
    names = list(dict.fromkeys(row["workload"] for row in rows))
    if len(rows) != 160 or len(names) != 10:
        raise ValueError("計測区間がそろっていない")
    for name in names:
        selected = [row for row in rows if row["workload"] == name]
        if len({(r["variant"], r["sample"]) for r in selected}) != 16:
            raise ValueError("重複または欠損した計測区間")
        for sample in range(8):
            pair = [r for r in selected if r["sample"] == sample]
            if len({r["checksum"] for r in pair}) != 1:
                raise ValueError("A/Bの計算結果が一致しない")
    result = {name: summarize([r for r in rows if r["workload"] == name]) for name in names}
    for family in ("replay", "expand"):
        result[family + "_all"] = summarize([r for r in rows if r["workload"].startswith(family + "_")])
    (directory / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    lines = ["# 塔A/Bの保存済み計測の集計", "", "単位は1操作または1候補あたりのCPU時間（ns）。値は8区間の中央値。", "",
             "| 処理 | 案A | 案B | B/A | 短い側と短縮率 | 区間ごとのB/A範囲 |", "|---|---:|---:|---:|---|---|",]
    for name in ("replay_all", "expand_all", "clone_apply_64", "clone_apply_8192", *names):
        if name.startswith("clone_apply") and lines and any(line.startswith("| " + name + " |") for line in lines):
            continue
        entry = result[name]
        a = entry["variants"]["A"]["median_cpu_ns_per_operation"]
        b = entry["variants"]["B"]["median_cpu_ns_per_operation"]
        ratios = entry["paired_ratios_B_over_A"]
        lines.append(f"| {name} | {a:.3f} | {b:.3f} | {b/a:.3f} | {entry['shorter']} {entry['time_reduction_percent']:.1f}% | {min(ratios):.3f}〜{max(ratios):.3f} |")
    lines.extend(["", "合算値は、区間ごとに3版のCPU時間と操作数をそれぞれ足してから割り、8区間の中央値を取った。",
                  "単独操作の2項目には値の読み出しと書き戻しが含まれる。フラグのXORだけを測った値ではない。", ""])
    (directory / "summary.md").write_text("\n".join(lines))
    print("\n".join(lines[:10]))


if __name__ == "__main__":
    main()
