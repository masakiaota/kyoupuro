#!/usr/bin/env python3
"""Verify saved v018 outputs and compare them without executing any solver."""

import hashlib
import json
import re
from pathlib import Path

from summarize_v010 import latest_run
from summarize_v012 import read_case, summarize_cases


ROOT = Path(__file__).resolve().parents[2]
BIN = "v018_relay_pickup"
LABEL = "retained_relay_pickup"
BASELINES = {
    "v015_mixed_tree": "pro_mixed_tree_port",
    "v017_timer_origin": "timer_origin_local",
}
LEASE = re.compile(
    r"^\[relay\.lease\] stage=(\d+) release=(\d+) row=(\d+) col=(\d+) keep=(\d+) mode=(\d+)$",
    re.MULTILINE,
)


def compare(cases, baselines):
    result = summarize_cases(cases, baselines)
    sums = result["counts_sum"]
    result["relay_mechanism_passed"] = all(sums.get(key, 0) > 0 for key in (
        "relay_trials", "relay_accepted", "relay_sites", "relay_output_saved", "relay_later_jumps",
    ))
    parent = baselines["v015_mixed_tree"]
    result["phase_differences_vs_v015"] = {
        "initial_best_ops": sum(c["counts"]["initial_best_ops"] - parent[c["case_name"]]["counts"]["initial_best_ops"] for c in cases),
        "before_relay": sum(c["counts"]["pre_relay_ops"] - parent[c["case_name"]]["counts"]["pre_branch_ops"] for c in cases),
        **{
            key: sum(c["counts"][key] - parent[c["case_name"]]["counts"][key] for c in cases)
            for key in ("pre_branch_ops", "branch_saved", "local_saved", "joint_saved", "final_ops")
        },
    }
    result["max_retention_stage_gap"] = max(c["counts"]["relay_max_stage_gap"] for c in cases)
    return result


def main():
    audit = json.loads((ROOT / "adhoc/v018_port_audit/static_verification.json").read_text())
    digest = hashlib.sha256((ROOT / f"src/bin/{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == audit["solver_sha256"], "Solver changed after the pre-evaluation audit"
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r["run_id"] for r in records if r["bin"] == BIN}) == 1
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert all(r["local"] and r["input_dir"] == "tools/in" for r in current)
    prior = {name: latest_run(records, name, label) for name, label in BASELINES.items()}
    baselines = {name: {r["case_name"]: read_case(r) for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r["case_name"] for r in current}
        assert all(c["verified"] and c["local"] for c in baseline.values())
    cases = [read_case(r) for r in current]
    for case in cases:
        if not case["verified"]:
            continue
        counts = case["counts"]
        assert counts["relay_output_saved"] == counts["pre_relay_ops"] - counts["pre_branch_ops"] >= 0
        assert counts["relay_plan_saved"] >= counts["relay_output_saved"]
        assert counts["relay_adopted"] == int(counts["relay_output_saved"] > 0)
        assert counts["relay_sites"] == sum(counts[f"relay_mode{m}_sites"] for m in range(4)) <= 5
        assert counts["relay_delayed_slimes"] == counts["relay_recovered_slimes"]
        assert 0 <= counts["relay_used_slimes"] <= counts["relay_delayed_slimes"] <= 3 * counts["relay_sites"]
        assert 0 <= counts["relay_necessary_jumps"] <= counts["relay_later_jumps"]
        assert 0 <= counts["relay_accepted"] <= counts["relay_trials"]
        path = ROOT / case["stdout_path"]
        leases = [dict(zip(("stage", "release", "row", "col", "keep", "mode"), map(int, row)))
                  for row in LEASE.findall(path.with_name(path.name + ".err").read_text())]
        case["adopted_leases_before_output_repair"] = leases
        assert len(leases) == counts["relay_sites"]
        assert all(lease["release"] > lease["stage"] and 1 <= lease["keep"] <= 3 for lease in leases)
        assert sum(lease["release"] - lease["stage"] for lease in leases) == counts["relay_stage_gap_sum"]
        assert max((lease["release"] - lease["stage"] for lease in leases), default=0) == counts["relay_max_stage_gap"]
    result = {
        "bin": BIN, "parent": "v015_mixed_tree", "label": LABEL,
        "run_id": current[0]["run_id"], "executed_at": current[0]["executed_at"],
        "baseline_run_ids": {name: run[0]["run_id"] for name, run in prior.items()},
        "solver_sha256": digest, "original_source_sha256": audit["original_sha256"],
        "all_100": compare(cases, baselines),
        "generated_99": compare([c for c in cases if c["case_name"] != "0000.txt"], baselines),
        "case0000": compare([c for c in cases if c["case_name"] == "0000.txt"], baselines),
        "cases": cases,
    }
    all_cases, generated = result["all_100"], result["generated_99"]
    result["adopt"] = (
        all_cases["verified_cases"] == all_cases["cases_under_2000_ms"] == 100
        and generated["relay_mechanism_passed"]
        and all_cases.get("comparisons", {}).get("v015_mixed_tree", {}).get("delta_T", 0) < 0
    )
    output = ROOT / "adhoc/v018_evaluation_summary.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)), "run_id": result["run_id"],
        "adopt": result["adopt"], "verified_cases": all_cases["verified_cases"],
        "mean_T": all_cases.get("mean_T"), "total_T": all_cases.get("total_T"),
        "comparisons": all_cases.get("comparisons"),
        "mean_elapsed_ms": all_cases["mean_elapsed_ms"], "max_elapsed_ms": all_cases["max_elapsed_ms"],
        "generated_99_mechanism_passed": generated["relay_mechanism_passed"],
        "generated_99_relay": {
            key: {"total": value, "cases": generated["cases_positive"][key]}
            for key, value in generated["counts_sum"].items() if key.startswith("relay_")
        },
        "phase_differences_vs_v015": all_cases["phase_differences_vs_v015"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
