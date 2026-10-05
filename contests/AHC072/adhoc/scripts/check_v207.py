#!/usr/bin/env python3
"""v207の入力固定と保存出力の検証・集計。solverは実行しない。"""
import csv
import hashlib
import json
from pathlib import Path
import re
import sys

from tune_v204 import mechanism

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v207_audit"
OUT = ROOT / "results/analysis/v207"
BIN = "v207_lazy_candidate_bank"
PARENT_RUN = "20261003T031601+0900_v204_initial_race_tuned_47557d"
BANK_KEYS = ("full_builds", "history_only_builds", "packet_only_builds",
             "score_refreshes", "stale_bank_selections")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def records():
    with (ROOT / "results/eval_records.jsonl").open() as f:
        return [json.loads(line) for line in f if PARENT_RUN in line or BIN in line]


def prepare():
    assert not (AUDIT / "frozen.json").exists()
    parent = [r for r in records() if r["run_id"] == PARENT_RUN]
    inputs = sorted((ROOT / "tools/in").glob("*.txt"))
    assert len(parent) == len(inputs) == 100
    assert {r["case_name"] for r in parent} == {p.name for p in inputs}
    prior = json.loads((ROOT / "results/tuning/v204/frozen.json").read_text())
    for p in inputs:
        assert sha(p) == prior[str(p.relative_to(ROOT))]
    parent_metrics = json.loads((ROOT / "results/analysis/v206/parent_metrics.json").read_text())
    metrics = {r["case"]: r for r in parent_metrics["details"]}
    fixed = [ROOT / f"src/bin/{BIN}.cpp", ROOT / "src/bin/v204_initial_race_tuned.cpp",
             AUDIT / "v206_lazy_candidate_bank.cpp", AUDIT / "preregistration.md",
             AUDIT / "registered_changes.json", AUDIT / "preprocessed.json",
             AUDIT / "v207_local", AUDIT / "v207_production", AUDIT / "builds.json",
             ROOT / "results/analysis/v206/parent_metrics.json", Path(__file__),
             ROOT / "scripts/eval.py", ROOT / "scripts/build_solver.sh", *inputs]
    for filename in ("tune_v204.py", "tune_v073.py", "check_v037_results.py", "check_v028_two_orders.py"):
        fixed.append(ROOT / "adhoc/scripts" / filename)
    for r in parent:
        assert r["local"] and r["input_dir"] == "tools/in" and r["status"] == "ok"
        answer = ROOT / "results/analysis/v206/validation200/previous_outputs/v204_initial_race_tuned" / r["case_name"]
        counts, times = mechanism(ROOT / "tools/in" / r["case_name"], answer, (5, 15))
        assert counts == metrics[r["case_name"]]["counts"]
        assert times == metrics[r["case_name"]]["times_ms"]
        assert counts["T"] == r["score"]
        fixed.extend((answer, answer.with_suffix(".txt.err")))
    save(AUDIT / "frozen.json", dict(
        sha256={str(p.relative_to(ROOT)): sha(p) for p in fixed}, parent_records=parent,
        jobs=2, local_time_ratio=0.80, input_dir="tools/in", cases=100,
        lns_end_ms=1520, search_limit_ms=1544, solver_executions=0))
    print("親の100出力と入力を照合し、ソース・両ビルド・検証器を固定した。")


def verify_frozen():
    frozen = json.loads((AUDIT / "frozen.json").read_text())
    for path, expected in frozen["sha256"].items():
        assert sha(ROOT / path) == expected, path
    return frozen


def inspect(case, answer):
    counts, times = mechanism(ROOT / "tools/in" / case, answer, (5, 15))
    assert times["search_limit"] == 1544.0
    line = next(line for line in answer.with_suffix(".txt.err").read_text().splitlines()
                if line.startswith("baseline="))
    values = dict(re.findall(r"(\w+)=([^\s]+)", line))
    bank = {key: int(values[key]) for key in BANK_KEYS}
    bank.update({key: float(values[key]) for key in (
        "full_build_seconds", "history_only_seconds", "packet_only_seconds")})
    return dict(case=case, counts=counts, times_ms=times, bank=bank,
                output_sha256=sha(answer), stderr_sha256=sha(answer.with_suffix(".txt.err")))


def aggregate(rows):
    assert all(sum(r["bank"][key] for r in rows) > 0 for key in BANK_KEYS)
    return dict(cases=len(rows), bank_totals={key: sum(r["bank"][key] for r in rows) for key in BANK_KEYS},
                attempts=sum(r["counts"]["lns_attempts"] for r in rows),
                average_times_ms={key: sum(r["times_ms"].get(key, 0) for r in rows)/len(rows)
                                  for key in ("construction", "temporal_lns", "lns_candidate_build", "total")},
                bank_average_times_ms={key: sum(r["bank"][key] for r in rows)*1000/len(rows)
                                       for key in ("full_build_seconds", "history_only_seconds", "packet_only_seconds")})


def diagnostic():
    verify_frozen()
    rows = [inspect(case, AUDIT / ("diagnostic_" + case)) for case in ("0000.txt", "0001.txt")]
    result = dict(passed=True, **aggregate(rows), details=rows)
    save(AUDIT / "diagnostic.json", result)
    print(json.dumps({k: v for k, v in result.items() if k != "details"}, ensure_ascii=False, indent=2))


def analyze():
    frozen = verify_frozen()
    assert json.loads((AUDIT / "diagnostic.json").read_text())["passed"]
    child = sorted((r for r in records() if r["bin"] == BIN), key=lambda r: r["case_name"])
    parent = {r["case_name"]: r for r in frozen["parent_records"]}
    assert len(child) == 100 and len({r["run_id"] for r in child}) == 1
    assert {r["case_name"] for r in child} == parent.keys()
    metrics, cases = [], []
    for r in child:
        assert r["local"] and r["input_dir"] == "tools/in" and r["status"] == "ok"
        m = inspect(r["case_name"], ROOT / r["stdout_path"])
        assert m["counts"]["T"] == r["score"]
        metrics.append(m)
        cases.append(dict(case=r["case_name"], parent_T=parent[r["case_name"]]["score"],
                          T=r["score"], delta_T=r["score"]-parent[r["case_name"]]["score"],
                          elapsed_ms=r["elapsed"], attempts=m["counts"]["lns_attempts"]))
    total, parent_total = sum(r["T"] for r in cases), sum(r["parent_T"] for r in cases)
    max_elapsed = max(r["elapsed_ms"] for r in cases)
    result = dict(run_id=child[0]["run_id"], parent_run_id=PARENT_RUN, total_sum=total,
                  total_avg=total/100, parent_sum=parent_total, delta_T=total-parent_total,
                  delta_percent=(total/parent_total-1)*100,
                  wins=sum(r["delta_T"] < 0 for r in cases), ties=sum(r["delta_T"] == 0 for r in cases),
                  losses=sum(r["delta_T"] > 0 for r in cases), max_elapsed_ms=max_elapsed,
                  avg_elapsed_ms=sum(r["elapsed_ms"] for r in cases)/100,
                  adopted=total < parent_total and max_elapsed <= 2000,
                  **aggregate(metrics))
    old = json.loads((ROOT / "results/analysis/v206/parent_metrics.json").read_text())
    result["parent_attempts"] = old["counts"]["lns_attempts"]
    result["parent_average_times_ms"] = {key: old["average_times_ms"][key] for key in result["average_times_ms"]}
    OUT.mkdir(parents=True, exist_ok=True)
    save(OUT / "comparison.json", result)
    save(OUT / "mechanism.json", metrics)
    with (OUT / "cases.csv").open("w") as f:
        writer = csv.DictWriter(f, fieldnames=list(cases[0]))
        writer.writeheader()
        writer.writerows(cases)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    {"prepare": prepare, "diagnostic": diagnostic, "analyze": analyze}[sys.argv[1]]()
