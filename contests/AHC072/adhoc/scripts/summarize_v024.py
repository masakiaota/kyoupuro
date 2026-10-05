#!/usr/bin/env python3
"""Verify and compare saved v024/control outputs; never run a solver."""
import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import compare, latest_run, read_case

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v024_audit"
RUNS = {
    "v021_paired_event_lns": "paired_event_lns",
    "v022_dependency_lns": "dependency_lns",
    "v024_incoming_reinsertion": "incoming_reinsertion",
    "ablate_v024_dependency": "incoming_without_dependency",
}


def summarize_fixed_work():
    rows = list(csv.DictReader((AUDIT / "fixed_work.csv").open()))
    assert len(rows) == 200 and all(r["equal"] == "1" for r in rows)
    generated = [r for r in rows if r["case"] != "0000"]
    result = {"all_100_equal": True}
    for kind in ("all", "0", "1"):
        chosen = [r for r in generated if kind == "all" or r["kind"] == kind]
        totals = {
            k: sum(float(r[k]) for r in chosen)
            for k in ("extract_calls", "insert_calls", "insert_success", "parent_extract_ns",
                      "new_extract_ns", "parent_insert_ns", "new_insert_ns")
        }
        totals["insertion_reduction_percent"] = 100 * (1 - totals["new_insert_ns"] / totals["parent_insert_ns"])
        totals["extraction_and_insertion_reduction_percent"] = 100 * (
            1 - (totals["new_insert_ns"] + totals["new_extract_ns"])
            / (totals["parent_insert_ns"] + totals["parent_extract_ns"])
        )
        result[kind] = totals
    result["all_100_totals"] = {
        k: sum(int(r[k]) for r in rows if r["kind"] == "0")
        for k in ("flexible_calls", "flexible_success", "paired_calls", "paired_success", "mask_checks")
    }
    result["speed_criterion_passed"] = result["all"]["insertion_reduction_percent"] >= 5
    (AUDIT / "fixed_work_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    audit = json.loads((AUDIT / "static_verification.json").read_text())
    for item in audit["files"].values():
        assert hashlib.sha256((ROOT / item["path"]).read_bytes()).hexdigest() == item["sha256"]
    fixed = summarize_fixed_work()
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    runs = {name: latest_run(records, name, label) for name, label in RUNS.items()}
    for name, run in runs.items():
        assert len(run) == len({r["case_name"] for r in run}) == 100
        assert all(r["local"] and r["input_dir"] == "tools/in" for r in run)
        if name in ("v024_incoming_reinsertion", "ablate_v024_dependency"):
            assert len({r["run_id"] for r in records if r["bin"] == name}) == 1
    assert runs["v021_paired_event_lns"][0]["run_id"] == "20260927T093556+0900_v021_paired_event_lns_987039"
    assert runs["v022_dependency_lns"][0]["run_id"] == "20260927T155946+0900_v022_dependency_lns_51c972"
    cases = {name: [read_case(r) for r in run] for name, run in runs.items()}
    indexes = {name: {c["case_name"]: c for c in group} for name, group in cases.items()}
    result = {"fixed_work": fixed, "run_ids": {name: run[0]["run_id"] for name, run in runs.items()}, "versions": {}}
    for name, group in cases.items():
        baselines = {n: v for n, v in indexes.items() if n != name}
        out = {
            "all_100": compare(group, baselines),
            "generated_99": compare([c for c in group if c["case_name"] != "0000.txt"], baselines),
            "case0000": compare([c for c in group if c["case_name"] == "0000.txt"], baselines),
            "cases": group,
        }
        out["pre_lns_differences"] = {
            other: {
                "cases": sum(c["counts"]["pre_lns_ops"] != before[c["case_name"]]["counts"]["pre_lns_ops"] for c in group),
                "total": sum(c["counts"]["pre_lns_ops"] - before[c["case_name"]]["counts"]["pre_lns_ops"] for c in group),
            }
            for other, before in baselines.items()
        }
        for label in ("all_100", "generated_99", "case0000"):
            counts = out[label]["counts_sum"]
            out[label]["loop_iterations"] = counts["lns_attempts"] + counts.get("lns_dependency_selected", 0) - counts.get("lns_dependency_generated", 0) + counts.get("lns_dependency_cooldown", 0)
        result["versions"][name] = out
    changed = result["versions"]["v024_incoming_reinsertion"]
    all_cases = changed["all_100"]
    result["mechanism_passed"] = all(all_cases["counts_sum"][key] > 0 for key in (
        "insert_incoming_seed_calls", "insert_incoming_candidates", "insert_incoming_height_updates"))
    result["adopt"] = (
        fixed["speed_criterion_passed"] and fixed["all_100_equal"] and result["mechanism_passed"]
        and all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == 0 and changed["case0000"]["total_T"] <= 43
        and all(all_cases["comparisons"][n]["delta_T"] < 0 for n in ("v021_paired_event_lns", "v022_dependency_lns"))
    )
    for c in cases["ablate_v024_dependency"]:
        assert c["counts"]["lns_dependency_selected"] == c["counts"]["lns_dependency_attempts"] == 0
    (ROOT / "adhoc/v024_evaluation_summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"adopt": result["adopt"], "speed_criterion_passed": fixed["speed_criterion_passed"],
        "mechanism_passed": result["mechanism_passed"], "run_ids": result["run_ids"], "versions": {
            n: {k: v["all_100"][k] for k in ("total_T", "mean_T", "verified_cases", "max_elapsed_ms", "error_count", "comparisons")}
            for n, v in result["versions"].items()
        }}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
