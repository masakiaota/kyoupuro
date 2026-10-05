#!/usr/bin/env python3
"""Summarize frozen measurements and independently replay the single evaluation."""
import csv
import json
import statistics
import sys

from run_v035_checks import unchanged, ROOT, OUT
import summarize_v034_capacity as timing
from summarize_v021 import latest_run, read_case
from summarize_v028 import group

BIN = "v035_tower_capacity_lns"
PARENT = "v028_two_order_lns"
LABEL = "tower_capacity"
timing.VARIANTS = ("original", "full", "compact")
timing.PAIRS = (("full", "original"), ("compact", "full"), ("compact", "original"))


def fixed_work():
    unchanged()
    rows = list(csv.DictReader((OUT / "fixed_work_samples.csv").open()))
    assert len(rows) == len({(r["round"], r["case"], r["variant"]) for r in rows}) == 1800
    cases = sorted({r["case"] for r in rows})
    assert len(cases) == 100
    case_results = []
    for case in cases:
        selected = [r for r in rows if r["case"] == case]
        assert len({(r["candidates"], r["completed"], r["checksum"]) for r in selected}) == 1
        for variant in timing.VARIANTS:
            assert {int(r["position"]) for r in selected if r["variant"] == variant and int(r["round"]) > 0} == {0, 1, 2}
        stats = timing.summarize(selected)
        item = {"case": case, "capacity": int(selected[0]["capacity"])}
        for metric in ("total_ns", "reconstruct_ns", "smooth_ns"):
            for name, value in stats[metric]["median_ns"].items():
                item[metric + "_" + name] = value
            for name, value in stats[metric]["comparisons"].items():
                item[metric + "_change_" + name] = value["change_percent"]
        case_results.append(item)
    normal = [r for r in rows if r["case"] != "0000.txt"]
    result = {"all_100": timing.summarize(rows), "normal_99": timing.summarize(normal)}
    result["capacity_groups"] = {str(cap): {"cases": len({r["case"] for r in normal if int(r["capacity"]) == cap}),
        "timing": timing.summarize([r for r in normal if int(r["capacity"]) == cap])}
        for cap in sorted({int(r["capacity"]) for r in normal})}
    result["case_counts"] = {}
    for new, base in timing.PAIRS:
        pair = new + "/" + base
        values = [r["total_ns_change_" + pair] for r in case_results if r["case"] != "0000.txt"]
        result["case_counts"][pair] = {"faster_by_1_percent": sum(x <= -1 for x in values),
            "within_1_percent": sum(-1 < x <= 1 for x in values), "slower_by_over_1_percent": sum(x > 1 for x in values),
            "min_percent": min(values), "max_percent": max(values)}
    with (OUT / "work_case_comparison.csv").open("w") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(case_results[0]))
        writer.writeheader()
        writer.writerows(case_results)
    sizes = list(csv.DictReader((OUT / "capacity_sizes.csv").open()))
    assert len(sizes) == 300
    result["memory"] = {}
    for variant in timing.VARIANTS:
        selected = [r for r in sizes if r["variant"] == variant and r["case"] != "0000.txt"]
        result["memory"][variant] = {k: statistics.mean(int(r[k]) for r in selected)
            for k in ("board_bytes", "identity_bytes", "dist_bytes", "geometry_bytes", "lns_bytes")}
    result["verification"] = {"cases": 100, "rows": len(rows), "variants": 3, "contents_equal": True,
        "source_hashes_unchanged": True, "execution_order_rotated": True,
        "candidates_per_round": sum(int(r["candidates"]) for r in rows if r["round"] == "1" and r["variant"] == "original")}
    (OUT / "fixed_work_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["normal_99"]["total_ns"], indent=2))


def evaluation():
    unchanged()
    fixed = json.loads((OUT / "fixed_clock_summary.json").read_text())
    work = json.loads((OUT / "fixed_work_summary.json").read_text())
    assert fixed["verified_cases"] == 100 and fixed["all_shared_counts_equal"] and fixed["rng_equal"] and fixed["ticks_equal"]
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    baseline = latest_run(records, PARENT, "two_order_lns")
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert baseline[0]["run_id"] == "20260927T184531+0900_v028_two_order_lns_c6311b"
    assert len(current) == 100 and all(r["status"] == "ok" and r["local"] for r in current)
    cases = [read_case(r) for r in current]
    old = {r["case_name"]: read_case(r) for r in baseline}
    baseline_map = {PARENT: {r["case_name"]: r for r in baseline}}
    sizes = {r["case"]: {k: int(r[k]) for k in ("capacity", "board_bytes", "identity_bytes", "geometry_bytes")}
             for r in csv.DictReader((OUT / "capacity_sizes.csv").open()) if r["variant"] == "compact"}
    for case in cases:
        lines = (ROOT / "tools/in" / case["case_name"]).read_text().splitlines()
        n = int(lines[0].split()[0])
        floors = sum(ch != "#" for row in lines[1:n + 1] for ch in row)
        counts, size = case["counts"], sizes[case["case_name"]]
        assert counts["floor_cells"] == floors
        assert counts["cell_capacity"] == size["capacity"] == min(400, (floors + 31) // 32 * 32)
        assert counts["board_bytes"] == size["board_bytes"] == 4 * counts["cell_capacity"]
        assert counts["identity_board_bytes"] == size["identity_bytes"] == 7200
        assert counts["geometry_bytes"] == size["geometry_bytes"] == 363952
    result = {"bin": BIN, "parent": PARENT, "run_id": current[0]["run_id"], "parent_run_id": baseline[0]["run_id"],
              "sources_unchanged": True, "capacity_verified_cases": len(cases), "fixed_clock_verified_cases": fixed["verified_cases"],
              "cases": cases}
    for name, predicate in (("all_100", lambda c: True), ("generated_99", lambda c: c != "0000.txt"), ("case0000", lambda c: c == "0000.txt")):
        selected = [c for c in cases if predicate(c["case_name"])]
        prior = [c for name, c in old.items() if predicate(name)]
        data, before = group(selected, baseline_map), group(prior, {})
        data["pre_lns_delta"] = sum(c["counts"]["pre_lns_ops"] - old[c["case_name"]]["counts"]["pre_lns_ops"] for c in selected)
        data["pre_lns_different_cases"] = sum(c["counts"]["pre_lns_ops"] != old[c["case_name"]]["counts"]["pre_lns_ops"] for c in selected)
        data["change_percent"] = {k: 100 * (data["counts_sum"][k] / before["counts_sum"][k] - 1)
                                  for k in ("lns_attempts", "single_insert_calls", "packet_insert_calls", "event_layers")}
        data["change_percent"]["loops_including_dependency_skips"] = 100 * (data["loops_including_dependency_skips"] / before["loops_including_dependency_skips"] - 1)
        result[name] = data
    all_cases = result["all_100"]
    errors = ("baseline_recovery", "final_recovery", "construction_errors", "lns_errors", "lns_invalid_candidates")
    result["required_passed"] = (all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
                                 and all(all_cases["counts_sum"].get(k, 0) == 0 for k in errors))
    comparisons = work["normal_99"]["total_ns"]["comparisons"]
    result["speed_noninferior"] = comparisons["compact/original"]["noninferior_within_1_percent"]
    result["structure_noninferior"] = comparisons["full/original"]["noninferior_within_1_percent"]
    result["speed_improved"] = comparisons["compact/original"]["faster_by_1_percent"]
    result["adopt_submission_candidate"] = (result["required_passed"] and result["speed_noninferior"] and result["structure_noninferior"]
        and all_cases["comparisons"][PARENT]["delta_T"] <= 0 and result["case0000"]["total_T"] <= 43)
    (OUT / "evaluation_summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    with (OUT / "evaluation_case_comparison.csv").open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("case", "floors", "capacity", "v028_T", "v035_T", "delta_T", "pre_lns_delta", "attempts_delta", "elapsed_ms"))
        for c in cases:
            p, counts = old[c["case_name"]], c["counts"]
            writer.writerow((c["case_name"], counts["floor_cells"], counts["cell_capacity"], p["score"], c["score"], c["score"] - p["score"],
                counts["pre_lns_ops"] - p["counts"]["pre_lns_ops"], counts["lns_attempts"] - p["counts"]["lns_attempts"], c["elapsed"]))
    concise = {k: result[k] for k in ("run_id", "required_passed", "speed_noninferior", "structure_noninferior", "speed_improved", "adopt_submission_candidate")}
    concise.update(comparison=all_cases["comparisons"][PARENT], max_elapsed_ms=all_cases["max_elapsed_ms"], case0000=result["case0000"]["total_T"],
                   pre_lns_delta=all_cases["pre_lns_delta"], loops_change=result["generated_99"]["change_percent"])
    print(json.dumps(concise, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    {"work": fixed_work, "eval": evaluation}[sys.argv[1]]()
