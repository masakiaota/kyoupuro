#!/usr/bin/env python3
"""v212の固定、部分問題検査、通常100件評価。analyzeは保存結果だけを読む。"""
import argparse
from collections import Counter
import csv
from datetime import datetime
import json
import os
from pathlib import Path
import subprocess
import sys

from build_v202_v203 import save, sha
from check_v037_results import verify_output
from check_v210 import inspect, aggregate
from prepare_v212 import ROOT, AUDIT, PARENT, CHILD, PARENT_RUN
from tune_v204 import evaluator

OUT = ROOT / "results/analysis/v212"
LABEL = "v212_mono_packet_quotient_same_local_budget"
ALIAS_KEYS = ("mono_calls", "closures", "gap_relaxations", "top_relaxations", "cost_relaxations", "completed")


def status(state, **extra):
    save(OUT / "status.json", dict(state=state, pid=os.getpid(),
        updated_at=datetime.now().astimezone().isoformat(), **extra))


def records():
    return [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()
            if CHILD in line or PARENT_RUN in line]


def freeze():
    assert not (AUDIT / "frozen.json").exists()
    pre = json.loads((AUDIT / "preflight.json").read_text())
    for name, expected in pre["solver_sha256"].items():
        assert sha(ROOT / f"src/bin/{name}.cpp") == expected
    for build in pre["builds"]:
        assert not build["warnings"]
        assert sha(AUDIT / f"{CHILD}_{build['mode']}") == build["binary_sha256"]
    assert (AUDIT / "probe_build.log").read_text() == ""
    inputs = evaluator.list_input_files(ROOT / "tools/in")
    assert len(inputs) == 100
    assert not any(row["bin"] == CHILD for row in records())
    rows = sorted(pre["parent_records"][PARENT], key=lambda row: row["case_name"])
    OUT.mkdir(parents=True, exist_ok=True)
    fixed = [*inputs, ROOT / f"src/bin/{PARENT}.cpp", ROOT / f"src/bin/{CHILD}.cpp",
             ROOT / "adhoc/bin/check_v212_packet_quotient.cpp", Path(__file__),
             ROOT / "adhoc/scripts/prepare_v212.py", ROOT / "scripts/eval.py", ROOT / "scripts/build_solver.sh"]
    fixed.extend(path for path in AUDIT.iterdir() if path.is_file())
    for name in ("check_v210.py", "check_v209.py", "check_v208.py", "check_v207.py",
                 "check_v037_results.py", "check_v028_two_orders.py", "tune_v204.py", "tune_v073.py"):
        fixed.append(ROOT / "adhoc/scripts" / name)
    details = []
    for row in rows:
        answer = ROOT / "results/out" / PARENT / row["case_name"]
        metric = inspect(row["case_name"], answer)
        assert metric["counts"]["T"] == row["score"]
        details.append(metric)
        fixed.extend((answer, answer.with_suffix(".txt.err")))
    parent_metrics = OUT / "parent_metrics.json"
    save(parent_metrics, aggregate(details)); fixed.append(parent_metrics)
    source = AUDIT / "reference_source.csv"
    source.write_bytes((ROOT / "results/score_detail.csv").read_bytes())
    reference_rows = list(csv.DictReader(source.open()))
    reference = {path.name: min(int(float(row[path.name])) for row in reference_rows
        if row.get(path.name) and float(row[path.name]) > 0) for path in inputs}
    fixed.append(source)
    save(AUDIT / "frozen.json", dict(
        sha256={str(path.relative_to(ROOT)): sha(path) for path in fixed},
        parent_records=rows, reference=reference, reference_versions=len(reference_rows),
        jobs=2, local_time_ratio=0.80, lns_end_ms=1520, search_limit_ms=1544,
        solver_executions_at_freeze=0))
    status("frozen")
    print("100入力、親の保存結果、相対参照、両ビルド、検証器を固定。solver実行0回。", flush=True)


def verify():
    frozen = json.loads((AUDIT / "frozen.json").read_text())
    for path, expected in frozen["sha256"].items():
        assert sha(ROOT / path) == expected, path
    return frozen


def diagnostic():
    verify()
    assert not (AUDIT / "diagnostic_started.json").exists()
    args = argparse.Namespace(bin_name=CHILD, jobs=2, wait_lock=True)
    with evaluator.acquire_eval_lock(args, "v212_diagnostic"):
        save(AUDIT / "diagnostic_started.json", dict(partial=True, smoke_case="0000.txt", modes=["local", "production"]))
        status("diagnostic")
        with (AUDIT / "partial.log").open("w") as log:
            subprocess.run([str(AUDIT / "check_packet"), str(ROOT)], cwd=ROOT,
                           stdout=log, stderr=subprocess.STDOUT, check=True, timeout=300)
        partial = json.loads((AUDIT / "partial_summary.json").read_text())
        assert partial["passed"]
        print(json.dumps(partial, ensure_ascii=False), flush=True)
        smoke = []
        for mode in ("local", "production"):
            answer = AUDIT / f"{mode}_0000.txt"
            input_path = ROOT / "tools/in/0000.txt"
            with input_path.open() as stdin, answer.open("w") as stdout, answer.with_suffix(".txt.err").open("w") as stderr:
                subprocess.run([str(AUDIT / f"{CHILD}_{mode}")], stdin=stdin,
                    stdout=stdout, stderr=stderr, check=True, timeout=15)
            score = verify_output(str(input_path), answer)
            assert "diagnostic:" not in answer.with_suffix(".txt.err").read_text()
            if mode == "local":
                metric = inspect("0000.txt", answer)
                assert metric["counts"]["T"] == score
            smoke.append(dict(mode=mode, T=score, output_sha256=sha(answer)))
            print(f"{mode} 0000: 合法、全帰巣", flush=True)
        save(AUDIT / "diagnostic.json", dict(passed=True, partial=partial, smoke=smoke))
        status("diagnostic_passed")


def analyze():
    frozen = verify()
    assert json.loads((AUDIT / "diagnostic.json").read_text())["passed"]
    rows = sorted((row for row in records() if row["bin"] == CHILD), key=lambda row: row["case_name"])
    assert len(rows) == 100 and len({row["run_id"] for row in rows}) == 1
    parent = {row["case_name"]: row for row in frozen["parent_records"]}
    assert set(parent) == {row["case_name"] for row in rows}
    details, cases = [], []
    for row in rows:
        assert row["status"] == "ok" and row["local"] and row["input_dir"] == "tools/in" and row["label"] == LABEL
        metric = inspect(row["case_name"], ROOT / row["stdout_path"])
        counts = metric["counts"]
        assert counts["T"] == row["score"]
        aliases = {key: counts["packet_alias_" + key] for key in ALIAS_KEYS}
        assert all(value >= 0 for value in aliases.values())
        assert aliases["cost_relaxations"] <= aliases["gap_relaxations"] + aliases["top_relaxations"]
        assert aliases["completed"] <= aliases["mono_calls"] <= counts["packet_insert_calls"]
        details.append(metric)
        old = parent[row["case_name"]]["score"]
        ref = frozen["reference"][row["case_name"]]
        grid = (ROOT / "tools/in" / row["case_name"]).read_text().splitlines()[1:]
        population = sum("a" <= char <= "l" for line in grid for char in line)
        cases.append(dict(case=row["case_name"], M=population, parent_T=old, T=row["score"],
            delta=row["score"]-old, reference_T=ref, relative_delta=100*ref*(1/row["score"]-1/old),
            elapsed_ms=row["elapsed"], **aliases))
    metrics = aggregate(details)
    aliases = {key: metrics["counts"]["packet_alias_"+key] for key in ALIAS_KEYS}
    active = aliases["mono_calls"] > 0 and aliases["closures"] > 0 and aliases["cost_relaxations"] > 0
    assert active, "packet quotient mechanism did not activate"
    total, old_total = sum(row["T"] for row in cases), sum(row["parent_T"] for row in cases)
    relative = sum(row["reference_T"] / row["T"] for row in cases)
    old_relative = sum(row["reference_T"] / row["parent_T"] for row in cases)
    parent_metrics = json.loads((OUT / "parent_metrics.json").read_text())
    parent_details = {row["case"]: row for row in parent_metrics["details"]}
    phases = {key: sum(row["counts"][key] - parent_details[row["case"]]["counts"][key] for row in details)
              for key in ("pre_lns_ops", "pre_pair_ops")}
    maximum = max(row["elapsed_ms"] for row in cases)
    result = dict(bin=CHILD, run_id=rows[0]["run_id"], parent_run=PARENT_RUN, cases=100,
        total=total, average=total/100, parent_total=old_total, parent_average=old_total/100,
        delta=total-old_total, delta_percent=100*(total/old_total-1),
        relative_average=relative, parent_relative_average=old_relative, relative_delta=relative-old_relative,
        wins=sum(row["delta"]<0 for row in cases), draws=sum(row["delta"]==0 for row in cases),
        losses=sum(row["delta"]>0 for row in cases), max_elapsed_ms=maximum,
        average_elapsed_ms=sum(row["elapsed_ms"] for row in cases)/100,
        aliases=aliases, active_cases={key: sum(row[key]>0 for row in cases) for key in ALIAS_KEYS},
        lns_attempts=metrics["counts"]["lns_attempts"], parent_lns_attempts=parent_metrics["counts"]["lns_attempts"],
        packet_insert_calls=metrics["counts"]["packet_insert_calls"], phase_deltas=phases,
        average_times_ms=metrics["average_times_ms"], parent_average_times_ms=parent_metrics["average_times_ms"],
        all_outputs_verified=True, all_errors_zero=True, mechanism_passed=active,
        adopted=total<old_total and relative>old_relative and maximum<=2000)
    result["strata"] = {}
    for name, group in (("M_lt_80", [row for row in cases if row["M"]<80]),
                        ("M_ge_80", [row for row in cases if row["M"]>=80])):
        result["strata"][name] = dict(cases=len(group), delta=sum(row["delta"] for row in group),
            relative_average_delta=sum(row["relative_delta"] for row in group)/len(group) if group else None)
    save(OUT / "comparison.json", result); save(OUT / "mechanism.json", metrics)
    with (OUT / "cases.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(cases[0])); writer.writeheader(); writer.writerows(cases)
    print(json.dumps({key: result[key] for key in ("run_id", "total", "parent_total", "delta", "relative_delta",
        "wins", "draws", "losses", "max_elapsed_ms", "aliases", "lns_attempts", "strata", "adopted")},
        ensure_ascii=False, indent=2), flush=True)
    return result


def run():
    verify()
    assert json.loads((AUDIT / "diagnostic.json").read_text())["passed"]
    assert not (OUT / "evaluation_started.json").exists()
    args = argparse.Namespace(bin_name=CHILD, jobs=evaluator.default_jobs(), wait_lock=True,
        dry_run=False, verbose=False, label=LABEL, no_local=False)
    with evaluator.acquire_eval_lock(args, "tools/in"):
        save(OUT / "evaluation_started.json", dict(cases=100, label=LABEL)); status("evaluating")
        assert evaluator.run_eval_locked(args, evaluator.list_input_files(ROOT / "tools/in"), "tools/in", True, True) == 0
    result = analyze(); status("completed", adopted=result["adopted"])


if __name__ == "__main__":
    try:
        {"freeze": freeze, "diagnostic": diagnostic, "run": run, "analyze": analyze}[sys.argv[1]]()
    except BaseException as error:
        if OUT.exists():
            status("failed", error=f"{type(error).__name__}: {error}")
        raise
