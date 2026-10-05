#!/usr/bin/env python3
"""保存済み評価だけを照合・集計する。solverは実行しない。"""
import collections
import csv
import hashlib
import json
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[2]
run = Path(sys.argv[1]).resolve()
stage = sys.argv[2]
manifest = json.loads((run / "manifest.json").read_text())
config = manifest["stages"][stage]
state = json.loads((run / f"{stage}_state.json").read_text())
assert state["status"] == "completed" and state["exit_code"] == 0
source = root / "src/bin" / f"{config['bin']}.cpp"
assert hashlib.sha256(source.read_bytes()).hexdigest() == config["source_sha256"]
records = [json.loads(line) for line in (root / "results/eval_records.jsonl").open()]
current = [r for r in records if r.get("label") == config["label"]]
cases = {p.name for p in (root / config["input_dir"]).rglob("*") if p.is_file()}
assert len(current) == len(cases) and {r["case_name"] for r in current} == cases
assert all(r["status"] == "ok" and r["input_dir"] == config["input_dir"] and r["local"] for r in current)
error_counts = collections.Counter()
complete = 0
active = 0
mechanism = collections.Counter()
for row in current:
    output = run / f"{stage}_outputs" / row["case_name"]
    stderr = output.with_name(output.name + ".err").read_text()
    counts = {key: int(value) for key, value in re.findall(
        r"^\[summary.count\] (\S+)=(-?\d+)$", stderr, re.M)}
    T = sum(bool(line.strip()) for line in output.read_text().splitlines())
    assert row["score"] == T + 100000 * counts["E"]
    if "final_ops" in counts:
        assert counts["final_ops"] == T
    complete += counts["E"] == 0
    active += counts.get("lns_attempts", 0) > 0
    if counts["E"] == 0:
        assert counts["state_pool_free_at_end"] == 4
    detail = {key: float(value) for key, value in re.findall(
        r"\b(v31[12]_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)", stderr)}
    if config["bin"] == "v311_neural_multistart":
        mechanism["multiple_seeds_cases"] += detail["v311_nn_retained"] > 1
        winner = int(detail["v311_winner"])
        mechanism["transformed_winner_cases"] += detail[f"v311_seed_{winner}_origin"] != -105
        mechanism["nonshortest_initial_winner_cases"] += winner > 0
        mechanism["completed_seeds"] += int(detail["v311_nn_completed"])
    else:
        mechanism["dp_active_cases"] += detail["v312_dp_attempts"] > 0
        mechanism["dp_attempts"] += int(detail["v312_dp_attempts"])
        mechanism["dp_completed"] += int(detail["v312_dp_completed"])
        mechanism["dp_improving_cases"] += detail["v312_dp_saved"] > 0
        mechanism["nn_complete_cases"] += detail["v312_nn_E"] == 0
        mechanism[f"winner_{int(detail['v312_winner'])}_cases"] += 1
    for key in ("construction_errors", "lns_errors", "baseline_recovery", "final_recovery"):
        assert key in counts
        error_counts[key] += counts[key]
is_in = config["input_dir"] == "tools/in"
baseline_labels = {
    "v105": "v105_in_j2_20261004T100932" if is_in else "v105_validation1_j1_20261004T100932",
    "v214": "v214_local_color_runs_flexible_paired_same_budget" if is_in
            else "validation1_j1_v208_v214_20261004T021012_744383",
}
if stage == "v312_in":
    baseline_labels["v311_standard"] = manifest["stages"]["v311_in"]["label"]
if stage == "v311_validation1":
    baseline_labels["v312"] = manifest["stages"]["v312_validation1"]["label"]
scores = {row["case_name"]: row["score"] for row in current}
baselines = {}
for name, label in baseline_labels.items():
    rows = [r for r in records if r.get("label") == label and
            (name != "v214" or r["bin"] == "v214_local_color_runs")]
    assert len(rows) == len(current) and all(r["status"] == "ok" for r in rows)
    values = {r["case_name"]: r["score"] for r in rows}
    assert values.keys() == scores.keys()
    baselines[name] = values
reference = manifest["reference"] if is_in else {
    case: min(scores[case], *(values[case] for values in baselines.values())) for case in scores}
relative = lambda values: 100 * sum(reference[k] / values[k] for k in scores) / len(scores)
total = sum(scores.values())
result = {"stage": stage, "label": config["label"], "cases": len(current), "sum": total,
          "average": total / len(current), "complete_cases": complete, "lns_active_cases": active,
          "error_counts": dict(error_counts), "avg_elapsed_ms": sum(r["elapsed"] for r in current) / len(current),
          "max_elapsed_ms": max(r["elapsed"] for r in current), "pseudo_relative": relative(scores),
          "relative_reference": manifest["reference_origin"] if is_in else
              "case minima of " + "/".join([stage.split("_")[0], *baselines]),
          "mechanism": dict(mechanism), "comparisons": {}}
for name, values in baselines.items():
    diffs = [scores[k] - values[k] for k in scores]
    previous_sum = sum(values.values())
    result["comparisons"][name] = {
        "label": baseline_labels[name], "average": previous_sum / len(current),
        "delta_sum": total - previous_sum, "improvement_pct": 100 * (previous_sum - total) / previous_sum,
        "wins": sum(d < 0 for d in diffs), "draws": sum(d == 0 for d in diffs), "losses": sum(d > 0 for d in diffs),
        "pseudo_relative": relative(values), "relative_delta_points": relative(scores) - relative(values)}
summary = [r for r in csv.DictReader((root / "results/score_summary.csv").open())
           if r["label"] == config["label"]]
assert len(summary) == 1 and int(summary[0]["total_sum"]) == total
(run / f"{stage}_result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(result, ensure_ascii=False))
