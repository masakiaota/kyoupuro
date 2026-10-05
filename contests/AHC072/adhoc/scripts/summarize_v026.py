#!/usr/bin/env python3
"""Check frozen v026 outputs and preregistered criteria without running a solver."""

import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import compare, latest_run, read_case

ROOT = Path(__file__).resolve().parents[2]
BIN = "v026_late_start_lns"
PARENT = "v022_dependency_lns"
LABEL = "late_start_lns"
KINDS = ("packet", "flexible", "paired")


def group(cases, baselines):
    result = compare(cases, baselines)
    counts = result["counts_sum"]
    result["late_start_mechanism_passed"] = (
        counts.get("lns_late_start_generated", 0) > 0
        and counts.get("lns_late_start_split_generated", 0) > 0
        and all(sum(counts.get(f"lns_late_start_{kind}_{key}", 0) for kind in KINDS[:2]) > 0
                for key in ("attempts", "completed", "accepted"))
    )
    result["late_start_direct_improved_cases"] = sum(
        sum(case["counts"].get(f"lns_late_start_{kind}_improvements", 0) for kind in KINDS[:2]) > 0
        for case in cases
    )
    result["late_start_any_improved_cases"] = sum(
        sum(case["counts"].get(f"lns_late_start_{kind}_improvements", 0) for kind in KINDS) > 0
        for case in cases
    )
    result["loops_including_dependency_skips"] = (
        counts["lns_attempts"] + counts["lns_dependency_selected"]
        - counts["lns_dependency_generated"] + counts["lns_dependency_cooldown"]
    )
    for case in cases:
        c, t = case["counts"], case["times_ms"]
        prefix = "lns_late_start_"
        assert 0 <= c[prefix + "split_generated"] <= c[prefix + "generated"] <= c[prefix + "primary"]
        assert c[prefix + "delay_sum"] >= c[prefix + "generated"]
        for kind in KINDS:
            d = {k.removeprefix(prefix + kind + "_"): v for k, v in c.items()
                 if k.startswith(prefix + kind + "_")}
            assert 0 <= d["improvements"] <= d["current_improvements"] <= d["accepted"] <= d["completed"] <= d["attempts"]
            assert 0 <= d["saved"] <= d["current_saved"]
            assert d["saved"] <= c[f"lns_{kind}_saved"]
        assert c[prefix + "packet_attempts"] + c[prefix + "flexible_attempts"] <= c["lns_block_attempts"]
        assert c[prefix + "packet_accepted"] + c[prefix + "flexible_accepted"] <= c["lns_block_accepted"]
        assert c[prefix + "flexible_attempts"] <= c["lns_flexible_attempts"]
        assert c[prefix + "paired_attempts"] <= c["lns_paired_attempts"]
        assert t[prefix + "packet"] + t[prefix + "flexible"] <= t["lns_block_search"] + 0.02
        assert t[prefix + "paired"] <= t["lns_paired_search"] + 0.02
        assert c["lns_dependency_generated"] == c["lns_dependency_attempts"] + c["lns_dependency_cooldown"]
        assert c["lns_dependency_improvements"] <= c["lns_dependency_accepted"] <= c["lns_dependency_completed"]
        assert c["lns_dependency_saved"] <= c["lns_regular_saved"]
    return result


def main():
    audit = json.loads((ROOT / "adhoc/v026_audit/static_verification.json").read_text())
    generation = json.loads((ROOT / "adhoc/v026_audit/generation_check.json").read_text())
    for name, key in ((BIN, "solver_sha256"), (PARENT, "parent_sha256")):
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = {
        PARENT: latest_run(records, PARENT, "dependency_lns"),
        "v021_paired_event_lns": latest_run(records, "v021_paired_event_lns", "paired_event_lns"),
    }
    assert prior[PARENT][0]["run_id"] == "20260927T155946+0900_v022_dependency_lns_51c972"
    baselines = {name: {r["case_name"]: r for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(r["local"] and r["status"] == "ok" for r in baseline.values())
    cases = [read_case(r) for r in current]
    parent_cases = {r["case_name"]: read_case(r) for r in prior[PARENT]}
    result = {
        "bin": BIN, "parents": ["v022"], "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "solver_sha256": audit["solver_sha256"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in prior.items()},
        "generation_verification": {k: v for k, v in generation.items() if k != "cases"},
        "all_100": group(cases, baselines),
        "generated_99": group([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": group([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "cases": cases,
    }
    all_cases, normal = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == 0 and generation["verified_cases"] == 100
        and normal["late_start_mechanism_passed"]
        and all_cases.get("comparisons", {}).get(PARENT, {}).get("delta_T", 0) < 0
        and result["case0000"].get("total_T", 100001) <= 43
    )
    result["pre_lns_differences"] = {
        "cases": sum(c["counts"]["pre_lns_ops"] != parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
        "total": sum(c["counts"]["pre_lns_ops"] - parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
    }
    parent_normal = [c for name, c in parent_cases.items() if name != "0000.txt"]
    result["parent_generated_99"] = compare(parent_normal, {})
    pc = result["parent_generated_99"]["counts_sum"]
    result["parent_generated_99"]["loops_including_dependency_skips"] = (
        pc["lns_attempts"] + pc["lns_dependency_selected"] - pc["lns_dependency_generated"] + pc["lns_dependency_cooldown"]
    )
    result["relative_changes_percent"] = {
        "lns_attempts": 100 * (normal["counts_sum"]["lns_attempts"] / pc["lns_attempts"] - 1),
        "candidate_build_time": 100 * (normal["mean_times_ms"]["lns_candidate_build"] /
                                      result["parent_generated_99"]["mean_times_ms"]["lns_candidate_build"] - 1),
    }
    output = ROOT / "adhoc/v026_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    with (ROOT / "adhoc/v026_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(("case", "v022_T", "v026_T", "delta_T", "pre_lns_delta", "v026_attempts",
                         "late_direct_best_saved", "late_paired_best_saved", "elapsed_ms"))
        for case in sorted(cases, key=lambda c: c["case_name"]):
            p, c = parent_cases[case["case_name"]], case["counts"]
            writer.writerow((case["case_name"], p["score"], case["score"], case["score"] - p["score"],
                             c["pre_lns_ops"] - p["counts"]["pre_lns_ops"], c["lns_attempts"],
                             c["lns_late_start_packet_saved"] + c["lns_late_start_flexible_saved"],
                             c["lns_late_start_paired_saved"], case["elapsed"]))
    print(json.dumps({
        "output": str(output), "run_id": result["run_id"], "adopt": result["adopt"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"), "verified_cases": all_cases["verified_cases"],
        "max_elapsed_ms": all_cases["max_elapsed_ms"], "error_count": all_cases["error_count"],
        "mechanism_passed": normal["late_start_mechanism_passed"],
        "direct_improved_cases": normal["late_start_direct_improved_cases"],
        "any_improved_cases": normal["late_start_any_improved_cases"],
        "pre_lns_differences": result["pre_lns_differences"],
        "relative_changes_percent": result["relative_changes_percent"],
        "late_counts": {k: v for k, v in normal["counts_sum"].items() if k.startswith("lns_late_start_")},
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
