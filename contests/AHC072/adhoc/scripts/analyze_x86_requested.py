#!/usr/bin/env python3
"""minibookの保存結果だけを検証する。solverは実行しない。"""
from pathlib import Path
import collections
import hashlib
import json
import re
import sys

run = Path(sys.argv[1]).resolve()
stage = sys.argv[2]
m = json.loads((run / "manifest.json").read_text())
root = Path(m["runtime_root"])
c = m["stages"][stage]
assert json.loads((run / f"{stage}_state.json").read_text())["status"] == "completed"
assert hashlib.sha256((root / "src/bin" / (c["bin"] + ".cpp")).read_bytes()).hexdigest() == c["source_sha256"]
assert {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (root / c["input_dir"]).glob("*.txt")} == m["input_sha256"]["validation1"]
records = [json.loads(x) for x in (root / "results/eval_records.jsonl").open()]
rows = [x for x in records if x.get("label") == c["label"]]
assert len(rows) == 500 and {x["case_name"] for x in rows} == m["reference"].keys()
assert all(x["status"] == "ok" and not x["local"] and x["bin"] == c["bin"] for x in rows)
errors = collections.Counter()
mechanism = collections.Counter()
hashes, seeds = {}, {}
for row in rows:
    output = run / (stage + "_outputs") / row["case_name"]
    stderr = output.with_name(output.name + ".err").read_text()
    T = sum(bool(x.strip()) for x in output.read_text().splitlines())
    assert row["score"] == T  # 公式score=T+100000EなのでE=0も確認できる。
    ordinary = dict(re.findall(r"\b(\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)", stderr))
    assert int(ordinary["best"]) == T
    errors["lns_diagnostic_messages"] += stderr.count("LNS diagnostic:")
    for outkey, logkey in (("lns_attempts", "lns_attempts"), ("lns_insertions", "insertions"),
                           ("lns_accepted", "accepted"), ("lns_improvements", "improvements")):
        mechanism[outkey] += int(ordinary[logkey])
    detail = dict(re.findall(r"\b(v(?:311|315|318)_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)", stderr))
    # この行は最初のNNのE=0を確認した後の経路でのみ出力される。
    assert int(detail["v311_nn_first_ops"]) > 0
    mechanism["history_active_cases"] += int(detail["v318_history_queries"]) > 0
    mechanism["race_active_cases"] += int(detail["v315_slices"]) > 0
    mechanism["additional_nn_incomplete"] += int(detail["v311_nn_incomplete"])
    hashes[row["case_name"]] = detail["v311_nn_first_hash"]
    n = int(detail["v311_nn_retained"])
    seeds[row["case_name"]] = [(detail[f"v311_seed_{i}_origin"], detail[f"v311_seed_{i}_raw"]) for i in range(n)]
scores = {x["case_name"]: x["score"] for x in rows}
relative = lambda values: 100 * sum(m["reference"][k] / values[k] for k in scores) / len(scores)
result = {"stage": stage, "label": c["label"], "cases": len(rows), "sum": sum(scores.values()),
          "average": sum(scores.values()) / len(rows), "pseudo_relative": relative(scores),
          "max_elapsed_ms": max(x["elapsed"] for x in rows), "avg_elapsed_ms": sum(x["elapsed"] for x in rows) / len(rows),
          "error_counts": dict(errors), "mechanism": dict(mechanism), "comparisons": {},
          "source_sha256": c["source_sha256"], "all_home": True, "all_first_nn_complete": True,
          "scores": scores, "first_nn_hashes": hashes, "seed_metadata": seeds,
          "unavailable_diagnostics": ["LOCAL summary.count final_ops", "state_pool_free_at_end", "CNN per-layer counts", "LOCAL error/recovery counters"]}
controls = {name + "_mac": data["scores"] for name, data in m["mac_baselines"].items()}
for other, config in m["stages"].items():
    saved = run / f"{other}_result.json"
    if other != stage and saved.exists():
        previous = json.loads(saved.read_text())
        controls[other] = previous["scores"]
        result["first_nn_hash_matches_remote_parent"] = sum(hashes[k] == previous["first_nn_hashes"][k] for k in hashes)
        result["seed_metadata_matches_remote_parent"] = sum(seeds[k] == [tuple(v) for v in previous["seed_metadata"][k]] for k in seeds)
        result["lns_attempts_delta_pct_vs_remote_parent"] = 100 * (mechanism["lns_attempts"] / previous["mechanism"]["lns_attempts"] - 1)
for name, values in controls.items():
    assert values.keys() == scores.keys()
    diffs = [scores[k] - values[k] for k in scores]
    result["comparisons"][name] = {"delta_sum": sum(diffs), "relative_delta_points": relative(scores) - relative(values),
        "wins": sum(x < 0 for x in diffs), "draws": sum(x == 0 for x in diffs), "losses": sum(x > 0 for x in diffs)}
assert not any(errors.values())
(run / f"{stage}_result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({k: v for k, v in result.items() if k not in ("scores", "first_nn_hashes", "seed_metadata")}, ensure_ascii=False))
