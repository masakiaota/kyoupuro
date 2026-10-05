#!/usr/bin/env python3
"""Verify frozen v027 outputs and its preregistered adoption conditions."""

import argparse
import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import latest_run, read_case
from summarize_v026 import group as late_start_group

ROOT = Path(__file__).resolve().parents[2]
BIN = "v027_resampled_start_lns"
PARENT = "v026_late_start_lns"
LABEL = "resampled_start_lns_rerun_after_sleep"
KINDS = ("packet", "flexible", "paired")
PREFIX = "lns_resampled_start_"


def group(cases, baselines):
    result = late_start_group(cases, baselines)
    c = result["counts_sum"]
    result["resampled_start_mechanism_passed"] = (
        c[PREFIX + "intermediate_generated"] > 0 and c[PREFIX + "alternative_generated"] > 0
        and all(sum(c[f"{PREFIX}{kind}_{key}"] for kind in KINDS[:2]) > 0
                for key in ("attempts", "completed", "accepted"))
    )
    result["alternative_direct_improved_cases"] = sum(
        sum(case["counts"][f"{PREFIX}{kind}_improvements"] for kind in KINDS[:2]) > 0 for case in cases)
    result["alternative_any_improved_cases"] = sum(
        sum(case["counts"][f"{PREFIX}{kind}_improvements"] for kind in KINDS) > 0 for case in cases)
    for case in cases:
        c, times = case["counts"], case["times_ms"]
        generated = c["lns_late_start_generated"]
        assert 0 <= c[PREFIX + "eligible_sets"] <= generated
        assert 0 <= c[PREFIX + "alternative_generated"] <= c[PREFIX + "eligible_sets"]
        assert 0 <= c[PREFIX + "intermediate_generated"] <= c[PREFIX + "eligible_sets"]
        assert c[PREFIX + "unique_later_times"] >= generated + c[PREFIX + "eligible_sets"]
        # An interrupted final build may have drawn samples before recording
        # its completed candidate bank, so permit those extra draws.
        assert c[PREFIX + "draws"] >= c[PREFIX + "unique_later_times"] - generated
        for kind in KINDS:
            d = {key: c[f"{PREFIX}{kind}_{key}"] for key in ("attempts", "completed", "accepted", "improvements", "saved")}
            assert 0 <= d["improvements"] <= d["accepted"] <= d["completed"] <= d["attempts"]
            for key, value in d.items():
                assert 0 <= value <= c[f"lns_late_start_{kind}_{key}"]
            assert 0 <= times[PREFIX + kind] <= times["lns_late_start_" + kind] + 0.02
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--label", default=LABEL)
    args = parser.parse_args()
    audit = json.loads((ROOT / "adhoc/v027_audit/static_verification.json").read_text())
    generation = json.loads((ROOT / "adhoc/v027_audit/generation_check.json").read_text())
    for name, key in ((BIN, "solver_sha256"), (PARENT, "parent_sha256")):
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, args.label)
    assert len({r["run_id"] for r in records if r["bin"] == BIN and r["label"] == args.label}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = {
        PARENT: latest_run(records, PARENT, "late_start_lns"),
        "v022_dependency_lns": latest_run(records, "v022_dependency_lns", "dependency_lns"),
    }
    assert prior[PARENT][0]["run_id"] == "20260927T172632+0900_v026_late_start_lns_973c99"
    baselines = {name: {r["case_name"]: r for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(r["local"] and r["status"] == "ok" for r in baseline.values())
    cases = [read_case(r) for r in current]
    parent_cases = {r["case_name"]: read_case(r) for r in prior[PARENT]}
    result = {
        "bin": BIN, "parents": ["v026"], "label": args.label,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "solver_sha256": audit["solver_sha256"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in prior.items()},
        "generation_verification": {k: v for k, v in generation.items() if k != "cases"},
        "all_100": group(cases, baselines),
        "generated_99": group([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": group([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "parent_generated_99": late_start_group([c for name, c in parent_cases.items() if name != "0000.txt"], {}),
        "cases": cases,
    }
    if args.label == LABEL:
        archive = ROOT / "adhoc/v027_audit/sleep_affected_run/v027_evaluation_summary.json"
        first_run = json.loads(archive.read_text())
        assert first_run["solver_sha256"] == result["solver_sha256"]
        result["evaluation_context"] = {
            "reason": "User reported one sleep during the initial evaluation and explicitly requested a rerun.",
            "source_unchanged": True,
            "excluded_initial_run_id": first_run["run_id"],
            "initial_run_archive": "adhoc/v027_audit/sleep_affected_run/",
            "decision_basis": "Rerun only, against the preregistered saved v026 baseline; no pooling or best-case selection.",
        }
    all_cases, normal = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and all_cases["error_count"] == 0 and generation["verified_cases"] == 100
        and normal["resampled_start_mechanism_passed"]
        and all_cases.get("comparisons", {}).get(PARENT, {}).get("delta_T", 0) < 0
        and result["case0000"].get("total_T", 100001) <= 43
    )
    result["pre_lns_differences"] = {
        "cases": sum(c["counts"]["pre_lns_ops"] != parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
        "total": sum(c["counts"]["pre_lns_ops"] - parent_cases[c["case_name"]]["counts"]["pre_lns_ops"] for c in cases),
    }
    parent_normal = result["parent_generated_99"]
    result["relative_changes_percent"] = {
        "lns_attempts": 100 * (normal["counts_sum"]["lns_attempts"] / parent_normal["counts_sum"]["lns_attempts"] - 1),
        "candidate_build_time": 100 * (normal["mean_times_ms"]["lns_candidate_build"] / parent_normal["mean_times_ms"]["lns_candidate_build"] - 1),
    }
    output = ROOT / "adhoc/v027_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    with (ROOT / "adhoc/v027_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(("case", "v026_T", "v027_T", "delta_T", "pre_lns_delta", "v027_attempts",
                         "alternative_direct_best_saved", "alternative_paired_best_saved", "elapsed_ms"))
        for case in sorted(cases, key=lambda c: c["case_name"]):
            p, c = parent_cases[case["case_name"]], case["counts"]
            writer.writerow((case["case_name"], p["score"], case["score"], case["score"] - p["score"],
                             c["pre_lns_ops"] - p["counts"]["pre_lns_ops"], c["lns_attempts"],
                             c[PREFIX + "packet_saved"] + c[PREFIX + "flexible_saved"],
                             c[PREFIX + "paired_saved"], case["elapsed"]))
    print(json.dumps({
        "output": str(output), "run_id": result["run_id"], "adopt": result["adopt"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"), "verified_cases": all_cases["verified_cases"],
        "max_elapsed_ms": all_cases["max_elapsed_ms"], "error_count": all_cases["error_count"],
        "mechanism_passed": normal["resampled_start_mechanism_passed"],
        "direct_improved_cases": normal["alternative_direct_improved_cases"],
        "any_improved_cases": normal["alternative_any_improved_cases"],
        "pre_lns_differences": result["pre_lns_differences"],
        "relative_changes_percent": result["relative_changes_percent"],
        "resampled_counts": {k: v for k, v in normal["counts_sum"].items() if k.startswith(PREFIX)},
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
