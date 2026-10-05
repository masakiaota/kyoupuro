#!/usr/bin/env python3
"""Summarize real-clock timing and the one v036 normal evaluation."""
import csv
import json
import statistics
import sys

from measure_v036 import ROOT, OUT, unchanged, digest
import summarize_v034_capacity as timing
from summarize_v021 import latest_run, read_case
from summarize_v028 import group

BIN = "v036_inline_color_lns"
BASELINES = {"v028_two_order_lns": ("two_order_lns", "20260927T184531+0900_v028_two_order_lns_c6311b"),
             "v035_tower_capacity_lns": ("tower_capacity", "20260928T004246+0900_v035_tower_capacity_lns_ab58a3")}
timing.VARIANTS = ("v028", "v035", "v036")
timing.PAIRS = (("v036", "v035"), ("v036", "v028"), ("v035", "v028"))


def fixed_work():
    unchanged()
    rows = list(csv.DictReader((OUT / "fixed_work_samples.csv").open()))
    assert len(rows) == len({(r["round"], r["case"], r["variant"]) for r in rows}) == 1800
    cases = sorted({r["case"] for r in rows})
    assert len(cases) == 100
    per_case = []
    for case in cases:
        selected = [r for r in rows if r["case"] == case]
        assert len({(r["candidates"], r["completed"], r["checksum"]) for r in selected}) == 1
        for variant in timing.VARIANTS:
            assert {int(r["position"]) for r in selected if r["variant"] == variant and int(r["round"]) > 0} == {0, 1, 2}
        stats = timing.summarize(selected)
        item = {"case": case, "capacity": int(selected[0]["capacity"])}
        for metric in ("build_ns", "reconstruct_ns", "smooth_ns", "total_ns"):
            for name, value in stats[metric]["median_ns"].items():
                item[metric + "_" + name] = value
            for name, value in stats[metric]["comparisons"].items():
                item[metric + "_change_" + name] = value["change_percent"]
        per_case.append(item)
    normal = [r for r in rows if r["case"] != "0000.txt"]
    result = {"all_100": timing.summarize(rows), "normal_99": timing.summarize(normal)}
    result["capacity_groups"] = {str(cap): {"cases": len({r["case"] for r in normal if int(r["capacity"]) == cap}),
        "timing": timing.summarize([r for r in normal if int(r["capacity"]) == cap])}
        for cap in sorted({int(r["capacity"]) for r in normal})}
    result["case_counts"] = {}
    for new, base in timing.PAIRS:
        pair = new + "/" + base
        values = [r["total_ns_change_" + pair] for r in per_case if r["case"] != "0000.txt"]
        result["case_counts"][pair] = {"faster_by_1_percent": sum(x <= -1 for x in values),
            "within_1_percent": sum(-1 < x <= 1 for x in values), "slower_by_over_1_percent": sum(x > 1 for x in values),
            "min_percent": min(values), "max_percent": max(values)}
    with (OUT / "work_case_comparison.csv").open("w") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(per_case[0]))
        writer.writeheader()
        writer.writerows(per_case)
    sizes = list(csv.DictReader((OUT / "capacity_sizes.csv").open()))
    assert len(sizes) == 300
    result["memory"] = {v: {k: statistics.mean(int(r[k]) for r in sizes if r["variant"] == v and r["case"] != "0000.txt")
        for k in ("board_bytes", "identity_bytes", "dist_bytes", "geometry_bytes", "lns_bytes")} for v in timing.VARIANTS}
    result["verification"] = {"cases": 100, "rows": len(rows), "variants": 3, "contents_equal": True,
        "sources_unchanged": True, "execution_order_rotated": True, "clock": "original steady_clock", "LOCAL": False,
        "candidates_per_round": sum(int(r["candidates"]) for r in rows if r["round"] == "1" and r["variant"] == "v028")}
    (OUT / "fixed_work_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["normal_99"]["total_ns"], indent=2))


def evaluation():
    unchanged()
    work = json.loads((OUT / "fixed_work_summary.json").read_text())
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, "inline_color")
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == 100 and all(r["status"] == "ok" and r["local"] for r in current)
    prior = {name: latest_run(records, name, label) for name, (label, _) in BASELINES.items()}
    for name, run in prior.items():
        assert run[0]["run_id"] == BASELINES[name][1]
    baselines = {name: {r["case_name"]: r for r in run} for name, run in prior.items()}
    cases = [read_case(r) for r in current]
    old = {name: {r["case_name"]: read_case(r) for r in run} for name, run in prior.items()}
    for case in cases:
        lines = (ROOT / "tools/in" / case["case_name"]).read_text().splitlines()
        n = int(lines[0].split()[0]); floors = sum(ch != "#" for row in lines[1:n + 1] for ch in row)
        c = case["counts"]
        assert c["floor_cells"] == floors
        assert c["cell_capacity"] == min(400, (floors + 31) // 32 * 32)
        assert c["board_bytes"] == 4 * c["cell_capacity"] and c["identity_board_bytes"] == 7200 and c["geometry_bytes"] == 363952
    result = {"bin": BIN, "run_id": current[0]["run_id"], "baseline_run_ids": {n: r[0]["run_id"] for n, r in prior.items()},
              "sources_unchanged": True, "cases": cases, "capacity_verified_cases": len(cases)}
    for title, pred in (("all_100", lambda c: True), ("generated_99", lambda c: c != "0000.txt"), ("case0000", lambda c: c == "0000.txt")):
        selected = [c for c in cases if pred(c["case_name"])]
        data = group(selected, baselines)
        data["behavior_comparisons"] = {}
        for name, before_map in old.items():
            before = group([c for k, c in before_map.items() if pred(k)], {})
            item = {"pre_lns_delta": sum(c["counts"]["pre_lns_ops"] - before_map[c["case_name"]]["counts"]["pre_lns_ops"] for c in selected),
                "pre_lns_different_cases": sum(c["counts"]["pre_lns_ops"] != before_map[c["case_name"]]["counts"]["pre_lns_ops"] for c in selected)}
            item["change_percent"] = {k: 100 * (data["counts_sum"][k] / before["counts_sum"][k] - 1)
                                     for k in ("lns_attempts", "single_insert_calls", "packet_insert_calls", "event_layers")}
            item["change_percent"]["loops_including_dependency_skips"] = 100 * (data["loops_including_dependency_skips"] / before["loops_including_dependency_skips"] - 1)
            data["behavior_comparisons"][name] = item
        result[title] = data
    all_cases = result["all_100"]
    errors = ("baseline_recovery", "final_recovery", "construction_errors", "lns_errors", "lns_invalid_candidates")
    result["required_passed"] = (all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all(all_cases["counts_sum"].get(k, 0) == 0 for k in errors) and result["case0000"]["total_T"] <= 43)
    result["speed_judgment"] = work["normal_99"]["total_ns"]["comparisons"]
    result["replace_v035_submission_candidate"] = (result["required_passed"] and all_cases["comparisons"]["v035_tower_capacity_lns"]["delta_T"] <= 0)
    # 再ビルドの日時差がある場合でも、ソースと命令の事前確認は保持する。
    result["local_binary_byte_identical"] = digest(ROOT / "target/release" / BIN) == digest(ROOT / "adhoc/v036_inline_color/local_solver")
    (OUT / "evaluation_summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    with (OUT / "evaluation_case_comparison.csv").open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("case", "v028_T", "v035_T", "v036_T", "delta_v028", "delta_v035", "elapsed_ms"))
        for c in cases:
            a, b = [old[n][c["case_name"]]["score"] for n in BASELINES]
            writer.writerow((c["case_name"], a, b, c["score"], c["score"] - a, c["score"] - b, c["elapsed"]))
    print(json.dumps({"run_id": result["run_id"], "required_passed": result["required_passed"],
        "replace_v035_submission_candidate": result["replace_v035_submission_candidate"], "comparisons": all_cases["comparisons"],
        "max_elapsed_ms": all_cases["max_elapsed_ms"], "case0000_T": result["case0000"]["total_T"],
        "behavior": result["generated_99"]["behavior_comparisons"], "local_binary_byte_identical": result["local_binary_byte_identical"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    {"work": fixed_work, "eval": evaluation}[sys.argv[1]]()
