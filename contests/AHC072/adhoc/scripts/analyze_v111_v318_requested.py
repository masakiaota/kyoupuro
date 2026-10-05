#!/usr/bin/env python3
"""保存済み公式評価・出力・診断を照合する。solverは実行しない。"""
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
dataset = config["input_dir"].split("/")[-1]
state = json.loads((run / f"{stage}_state.json").read_text())
assert state["status"] == "completed" and state["exit_code"] == 0
source = root / "src/bin" / f"{config['bin']}.cpp"
assert hashlib.sha256(source.read_bytes()).hexdigest() == config["source_sha256"]
input_hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                for p in (root / config["input_dir"]).glob("*.txt")}
assert input_hashes == manifest["input_sha256"][dataset]
records = [json.loads(line) for line in (root / "results/eval_records.jsonl").open()]
current = [r for r in records if r.get("label") == config["label"]]
assert len(current) == len(input_hashes) and {r["case_name"] for r in current} == input_hashes.keys()
assert all(r["status"] == "ok" and r["bin"] == config["bin"] and
           r["input_dir"] == config["input_dir"] and r["local"] for r in current)
error_counts = collections.Counter()
mechanism = collections.Counter()
complete = active = nn_complete = 0
for row in current:
    output = run / f"{stage}_outputs" / row["case_name"]
    stderr = output.with_name(output.name + ".err").read_text()
    counts = {key: int(value) for key, value in re.findall(
        r"^\[summary.count\] (\S+)=(-?\d+)$", stderr, re.M)}
    T = sum(bool(line.strip()) for line in output.read_text().splitlines())
    assert row["score"] == T + 100000 * counts["E"]
    assert counts["final_ops"] == T
    complete += counts["E"] == 0
    active += counts.get("lns_attempts", 0) > 0
    nn_complete += counts.get("nn_complete", 0) == 1
    assert counts["state_pool_free_at_end"] == 4
    detail = {key: float(value) for key, value in re.findall(
        r"\b(v(?:311|315|318)_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)", stderr)}
    mechanism["multiple_seeds_cases"] += detail["v311_nn_retained"] > 1
    mechanism["nn_incomplete"] += int(detail["v311_nn_incomplete"])
    winner = int(detail["v311_winner"])
    mechanism["nonoriginal_winner_cases"] += detail[f"v311_seed_{winner}_origin"] != -105
    version = stage.split("_")[0]
    if version == "v111":
        assert counts["cnn_full"] + counts["cnn_incremental"] == counts["cnn_calls"]
        assert counts["cnn_home"] <= counts["cnn_full"]
        for i in range(5):
            assert counts[f"cnn_cells_{i}"] <= counts["cnn_full_cells_per_layer"]
        for key, value in counts.items():
            if key.startswith("cnn_"):
                mechanism[key] += value
        mechanism["incremental_active_cases"] += counts["cnn_incremental"] > 0
    else:
        for key, value in detail.items():
            if key.startswith(("v315_", "v318_")):
                if key in ("v315_pilot_winner", "v315_final_winner"):
                    mechanism[f"{key}_{int(value)}_cases"] += 1
                elif key == "v318_peak_index_bytes":
                    mechanism["v318_peak_index_bytes_max"] = max(mechanism["v318_peak_index_bytes_max"], int(value))
                else:
                    mechanism[key] += value if key.endswith("_seconds") else int(value)
        mechanism["history_active_cases"] += detail["v318_history_queries"] > 0
        mechanism["feature_reuse_active_cases"] += detail["v318_feature_reuses"] > 0
        mechanism["reactive_race_active_cases"] += detail["v315_slices"] > 0
    raw_detail = dict(re.findall(r"\b(v311_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)", stderr))
    base_path = Path(manifest["baseline_v315_output_dirs"][dataset]) / (row["case_name"] + ".err")
    base_detail = dict(re.findall(r"\b(v311_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)", base_path.read_text()))
    mechanism["first_nn_hash_matches_v315_cases"] += raw_detail["v311_nn_first_hash"] == base_detail["v311_nn_first_hash"]
    n = int(raw_detail["v311_nn_retained"])
    bn = int(base_detail["v311_nn_retained"])
    mechanism["retained_seed_count_differs_from_v315_cases"] += n != bn
    seed_metadata = lambda d, length: [(d[f"v311_seed_{i}_origin"], d[f"v311_seed_{i}_raw"]) for i in range(length)]
    mechanism["same_initial_seed_metadata_as_v315_cases"] += seed_metadata(raw_detail, n) == seed_metadata(base_detail, bn)
    for key in ("construction_errors", "lns_errors", "baseline_recovery", "final_recovery"):
        error_counts[key] += counts[key]
scores = {row["case_name"]: row["score"] for row in current}
labels = dict(manifest["baseline_labels"][dataset])
# 先行する今回の版も、同じ事前固定参照で対比較する。
for other, other_config in manifest["stages"].items():
    other_state = run / f"{other}_state.json"
    if other != stage and other_config["input_dir"] == config["input_dir"] and other_state.exists():
        if json.loads(other_state.read_text())["status"] == "completed":
            labels[other.split("_")[0]] = other_config["label"]
baselines = {}
for name, label in labels.items():
    rows = [r for r in records if r.get("label") == label and
            (name != "v214" or r["bin"] == "v214_local_color_runs")]
    assert len(rows) == len(current) and all(r["status"] == "ok" for r in rows)
    values = {r["case_name"]: r["score"] for r in rows}
    assert values.keys() == scores.keys()
    baselines[name] = values
reference = manifest["reference"][dataset]
assert reference.keys() == scores.keys()
relative = lambda values: 100 * sum(reference[k] / values[k] for k in scores) / len(scores)
total = sum(scores.values())
result = {"stage": stage, "label": config["label"], "source_sha256": config["source_sha256"],
          "jobs": config["jobs"], "local_time_ratio": manifest["local_time_ratio"],
          "cases": len(current), "sum": total, "average": total / len(current),
          "complete_cases": complete, "nn_complete_cases": nn_complete, "lns_active_cases": active, "error_counts": dict(error_counts),
          "avg_elapsed_ms": sum(r["elapsed"] for r in current) / len(current),
          "max_elapsed_ms": max(r["elapsed"] for r in current), "pseudo_relative": relative(scores),
          "relative_reference": manifest["reference_origin"][dataset],
          "mechanism": dict(mechanism), "comparisons": {}, "single_saved_run_comparison": True}
for name, values in baselines.items():
    diffs = [scores[k] - values[k] for k in scores]
    previous_sum = sum(values.values())
    result["comparisons"][name] = {
        "label": labels[name], "average": previous_sum / len(current),
        "delta_sum": total - previous_sum, "improvement_pct": 100 * (previous_sum - total) / previous_sum,
        "wins": sum(d < 0 for d in diffs), "draws": sum(d == 0 for d in diffs), "losses": sum(d > 0 for d in diffs),
        "pseudo_relative": relative(values), "relative_delta_points": relative(scores) - relative(values)}
summary = [r for r in csv.DictReader((root / "results/score_summary.csv").open())
           if r["label"] == config["label"]]
assert len(summary) == 1 and int(summary[0]["total_sum"]) == total
result["summary_csv_rounded_average"] = summary[0]["total_avg"]
primary = manifest["primary_baseline"]
result["passes_baseline_criterion"] = (complete == nn_complete == len(current) and not any(error_counts.values())
    and result["max_elapsed_ms"] <= 2000 and result["comparisons"][primary]["delta_sum"] <= 0
    and result["comparisons"][primary]["relative_delta_points"] > 0)
(run / f"{stage}_result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(result, ensure_ascii=False))
