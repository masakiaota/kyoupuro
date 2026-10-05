#!/usr/bin/env python3
"""Summarize the frozen five-condition measurement; never runs a solver."""
import csv
import hashlib
import json
from pathlib import Path
import statistics

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v034_capacity_factors"
VARIANTS = ("original", "full", "tower", "other", "compact")
PAIRS = (("full", "original"), ("tower", "full"), ("compact", "other"),
         ("other", "full"), ("compact", "full"), ("compact", "original"), ("tower", "original"))
METRICS = ("build_ns", "reconstruct_ns", "smooth_ns", "total_ns",
           "build_wall_ns", "reconstruct_wall_ns", "smooth_wall_ns", "total_wall_ns")


def summarize(rows):
    result = {}
    for metric in METRICS:
        rounds = {variant: [sum(int(row[metric]) for row in rows
                               if row["variant"] == variant and int(row["round"]) == r)
                            for r in range(1, 6)] for variant in VARIANTS}
        medians = {name: statistics.median(values) for name, values in rounds.items()}
        pairs = {}
        for new, base in PAIRS:
            ratios = [b / a for a, b in zip(rounds[base], rounds[new])]
            ratio = medians[new] / medians[base]
            shorter = sum(r < 1 for r in ratios)
            pairs[new + "/" + base] = {
                "change_percent": (ratio - 1) * 100,
                "round_changes_percent": [(r - 1) * 100 for r in ratios],
                "faster_rounds": shorter,
                "noninferior_within_1_percent": ratio <= 1.01 and max(ratios) <= 1.01,
                "faster_by_1_percent": ratio <= 0.99 and shorter >= 4,
            }
        result[metric] = {"round_totals_ns": rounds, "median_ns": medians, "comparisons": pairs}
    return result


def main():
    rows = list(csv.DictReader((OUT / "fixed_work_samples.csv").open()))
    assert len(rows) == 3000
    assert len({(r["round"], r["case"], r["variant"]) for r in rows}) == 3000
    cases = sorted({row["case"] for row in rows})
    assert len(cases) == 100
    for case in cases:
        one = [r for r in rows if r["case"] == case]
        assert len({(r["candidates"], r["completed"], r["checksum"]) for r in one}) == 1
        for variant in VARIANTS:
            assert {int(r["position"]) for r in one if r["variant"] == variant and int(r["round"]) > 0} == set(range(5))
    normal = [r for r in rows if r["case"] != "0000.txt"]
    result = {"all_100": summarize(rows), "normal_99": summarize(normal)}
    result["capacity_groups"] = {}
    for capacity in sorted({int(r["capacity"]) for r in normal}):
        group = [r for r in normal if int(r["capacity"]) == capacity]
        result["capacity_groups"][str(capacity)] = {
            "cases": len({r["case"] for r in group}), "timing": summarize(group)}
    case_results = []
    for case in cases:
        group = [r for r in rows if r["case"] == case]
        stats = summarize(group)
        item = {"case": case, "capacity": int(group[0]["capacity"])}
        for metric in ("total_ns", "reconstruct_ns", "smooth_ns"):
            for name, value in stats[metric]["median_ns"].items():
                item[metric + "_" + name] = value
            for name, value in stats[metric]["comparisons"].items():
                item[metric + "_change_" + name] = value["change_percent"]
        case_results.append(item)
    with (OUT / "case_comparison.csv").open("w") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(case_results[0]))
        writer.writeheader()
        writer.writerows(case_results)
    result["case_counts"] = {}
    for new, base in PAIRS:
        key = "total_ns_change_" + new + "/" + base
        values = [r[key] for r in case_results if r["case"] != "0000.txt"]
        result["case_counts"][new + "/" + base] = {
            "faster_by_1_percent": sum(x <= -1 for x in values),
            "within_1_percent": sum(-1 < x <= 1 for x in values),
            "slower_by_over_1_percent": sum(x > 1 for x in values),
            "min_percent": min(values), "max_percent": max(values)}
    sizes = list(csv.DictReader((OUT / "capacity_sizes.csv").open()))
    assert len(sizes) == 500
    result["memory"] = {}
    for variant in VARIANTS:
        group = [r for r in sizes if r["variant"] == variant and r["case"] != "0000.txt"]
        result["memory"][variant] = {key: statistics.mean(int(r[key]) for r in group)
                                     for key in ("board_bytes", "identity_bytes", "dist_bytes", "geometry_bytes", "lns_bytes")}
    manifest = json.loads((OUT / "manifest.json").read_text())
    for section in ("originals", "frozen"):
        for name, expected in manifest[section].items():
            # The experiment note gains results after measurement; its preregistration stays frozen.
            path = OUT / "preregistration.md" if name == "notes/experiments/v034.md" else ROOT / name
            if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                raise RuntimeError("Changed during measurement: " + name)
    result["verification"] = {"rows": len(rows), "cases": 100, "variants": 5,
                              "contents_equal": True, "execution_positions_balanced": True,
                              "source_hashes_unchanged": True,
                              "candidates_per_round": sum(int(r["candidates"]) for r in rows
                                                          if r["round"] == "1" and r["variant"] == "original")}
    result["environment"] = {"cpu": manifest["cpu"], "compiler": manifest["compiler"]}
    (OUT / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    lines = ["# v034の固定候補測定", "", "通常99ケースを合計した5巡の中央値。単位はms。", "",
             "| 条件 | 候補生成 | 除去と再挿入 | 経路短縮 | 合計 |", "|---|---:|---:|---:|---:|"]
    for variant in VARIANTS:
        values = [result["normal_99"][metric]["median_ns"][variant] / 1e6 for metric in METRICS[:4]]
        lines.append("| " + variant + " | " + " | ".join(f"{v:.3f}" for v in values) + " |")
    lines += ["", "| 比較 | 合計時間の増減 | 短かった巡回 | 1%以内の非劣性 |", "|---|---:|---:|---|"]
    for name, stats in result["normal_99"]["total_ns"]["comparisons"].items():
        lines.append(f"| {name} | {stats['change_percent']:+.3f}% | {stats['faster_rounds']} / 5 | {stats['noninferior_within_1_percent']} |")
    (OUT / "summary.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    print(json.dumps(result["verification"], ensure_ascii=False))


if __name__ == "__main__":
    main()
