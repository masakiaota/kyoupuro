#!/usr/bin/env python3
"""Inspect the single saved v016 evaluation; never execute a solver."""

import hashlib
import json
from pathlib import Path
import re

from replay_slime_output import replay
from summarize_v012 import read_case, summarize_cases


ROOT = Path(__file__).resolve().parents[2]
BIN = "v016_continuous_tree"
BASELINES = ("v013_unified", "v014_joint_delivery", "v015_mixed_tree")


def compare_witness():
    """Compare two existing segments, without constructing a replacement."""
    current = replay(ROOT / "tools/in/0000.txt", ROOT / f"results/out/{BIN}/0000.txt")
    witness = replay(ROOT / "tools/in/0000.txt", ROOT / "adhoc/x_uta_case0000/tsukammo_43.txt")

    def states(trace):
        lines = (ROOT / "tools/in/0000.txt").read_text().splitlines()
        N = int(lines[0].split()[0])
        board = {(i, j): char if char.islower() else ""
                 for i, row in enumerate(lines[1:N + 1]) for j, char in enumerate(row) if char != "#"}
        result = [dict(board)]
        for operation in trace["operations"]:
            for name in ("source", "destination"):
                cell = tuple(operation[name])
                board[cell] = operation["after"][str(cell)]
            result.append(dict(board))
        return result

    a, b = states(current), states(witness)
    without_red = lambda state: {cell: word.replace("a", "") for cell, word in state.items()}
    assert without_red(a[32]) == without_red(b[28])
    assert without_red(a[38]) == without_red(b[32])
    assert [op["action"] for op in current["operations"][38:]] == [op["action"] for op in witness["operations"][32:39]]
    result = {
        "source": "Existing legal outputs only; no replacement sequence generated.",
        "comparison": {"v016_turns": [33, 38], "v016_moves": 6,
                       "witness_turns": [29, 32], "witness_moves": 4,
                       "same_nonred_state_before": True, "same_nonred_state_after": True,
                       "same_remaining_blue_route": True},
        "v016_segment": current["operations"][32:38],
        "witness_segment": witness["operations"][28:32],
    }
    (ROOT / "adhoc/v016_case0000_witness_comparison.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    return result["comparison"]


def main():
    digest = hashlib.sha256((ROOT / "src/bin" / f"{BIN}.cpp").read_bytes()).hexdigest()
    assert digest == (ROOT / "adhoc/v016_source_sha256.txt").read_text().strip()
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    runs = {}
    for version in (*BASELINES, BIN):
        found = [record for record in records if record["bin"] == version]
        assert found, version
        if version == BIN:
            assert len({record["run_id"] for record in found}) == 1
        latest = found[-1]["run_id"]
        selected = [record for record in found if record["run_id"] == latest]
        assert len(selected) == len({record["case_name"] for record in selected}) == 100
        assert all(record["local"] and record["input_dir"] == "tools/in" for record in selected)
        runs[version] = {record["case_name"]: read_case(record) for record in selected}
    cases = list(runs[BIN].values())
    for case in cases:
        counts = case["counts"]
        assert counts.get("cooperative_plan_trials", 0) == (
            counts.get("cooperative_plan_valid", 0) + counts.get("cooperative_plan_invalid", 0))
        assert counts.get("cooperative_plan_valid", 0) == (
            counts.get("cooperative_plan_no_gain", 0) + counts.get("cooperative_wins", 0))
        if not case["verified"]:
            continue
        trace = replay(ROOT / "tools/in" / case["case_name"], ROOT / case["stdout_path"])
        assert trace["metrics"]["E"] == 0 and trace["metrics"]["T"] == case["score"]
        case["replay_metrics"] = trace["metrics"]
        if case["case_name"] == "0000.txt":
            (ROOT / "adhoc/v016_case0000_trace.json").write_text(json.dumps(trace, ensure_ascii=False, indent=2) + "\n")
    report = {
        "bin": BIN, "solver_sha256": digest,
        "run_ids": {version: next(iter(run.values()))["run_id"] for version, run in runs.items()},
        "cases": cases,
    }
    for name, selected in (("all100", cases), ("normal99", [c for c in cases if c["case_name"] != "0000.txt"]),
                           ("case0000", [c for c in cases if c["case_name"] == "0000.txt"])):
        summary = summarize_cases(selected, {version: runs[version] for version in BASELINES})
        summary["phase_differences"] = {
            version: {key: sum(case["counts"].get(key, 0) - runs[version][case["case_name"]]["counts"].get(key, 0)
                               for case in selected)
                      for key in ("initial_best_ops", "pre_branch_ops", "branch_saved", "local_saved", "joint_saved", "final_ops")}
            for version in BASELINES}
        summary["cooperative_mean_ms"] = sum(c["times_ms"].get("cooperative", 0) for c in selected) / len(selected)
        report[name] = summary
    counts = report["normal99"]["counts_sum"]
    report["mechanism"] = {
        key: {"total": value, "cases": report["normal99"]["cases_positive"].get(key, 0)}
        for key, value in counts.items() if key.startswith("cooperative_")}
    report["mechanism_passed"] = all(counts.get(key, 0) > 0 for key in (
        "cooperative_completed", "cooperative_wins", "cooperative_incoming_merges",
        "cooperative_split_noninitial", "cooperative_carry_continuations"))
    report["adopt"] = (report["all100"]["verified_cases"] == 100
                       and report["all100"]["cases_under_2000_ms"] == 100
                       and report["mechanism_passed"]
                       and report["normal99"]["comparisons"]["v015_mixed_tree"]["delta_T"] < 0)
    report["large_improvement"] = {
        "case0000_target": report["case0000"].get("total_T", 100000) <= 44,
        "normal99_target": report["normal99"].get("total_T", 100000)
        <= 0.99 * sum(c["score"] for n, c in runs["v015_mixed_tree"].items() if n != "0000.txt"),
    }
    first = runs[BIN]["0000.txt"]
    log = (ROOT / (first["stdout_path"] + ".err")).read_text()
    report["case0000_candidate_log"] = [line for line in log.splitlines() if line.startswith("[cooperative.")]
    report["case0000_candidates"] = [
        {key: int(value) if key != "ids" else [int(x) for x in value.split(",")]
         for key, value in re.findall(r"(\w+)=([^ ]+)", line)}
        for line in report["case0000_candidate_log"] if line.startswith("[cooperative.candidate]")]
    report["case0000_witness_comparison"] = compare_witness()
    report["adoption_subsets"] = {}
    for label, adopted in (("adopted", True), ("not_adopted", False)):
        subset = [case for case in cases if case["case_name"] != "0000.txt"
                  and (case["counts"].get("cooperative_wins", 0) > 0) == adopted]
        report["adoption_subsets"][label] = summarize_cases(subset, {"v015_mixed_tree": runs["v015_mixed_tree"]})
    output = ROOT / "adhoc/v016_evaluation_analysis.json"
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({
        "output": str(output.relative_to(ROOT)), "run_ids": report["run_ids"],
        "adopt": report["adopt"], "mechanism_passed": report["mechanism_passed"],
        "large_improvement": report["large_improvement"],
        "verified_cases": report["all100"]["verified_cases"],
        "total_T": report["all100"].get("total_T"),
        "comparisons": report["all100"].get("comparisons"),
        "max_elapsed_ms": report["all100"]["max_elapsed_ms"],
        "case0000_T": report["case0000"].get("total_T"),
        "case0000_cooperative": {k: v for k, v in report["case0000"]["counts_sum"].items() if k.startswith("cooperative_")},
        "case0000_candidates": report["case0000_candidates"],
        "mechanism_normal99": report["mechanism"],
        "phases_vs_v015": report["normal99"]["phase_differences"]["v015_mixed_tree"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
