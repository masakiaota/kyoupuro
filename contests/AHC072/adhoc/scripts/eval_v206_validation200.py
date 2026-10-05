#!/usr/bin/env python3
"""固定したv204/v206を新規200件で一度ずつ評価する。analyzeは保存結果だけを読む。"""
import argparse
from collections import Counter
from datetime import datetime
import csv
import json
from pathlib import Path
import shutil
import subprocess
import sys

from build_v202_v203 import save, sha
from check_v206 import aggregate, mechanism as child_mechanism
from prepare_v206 import CHILD, PARENT, ROOT
from tune_v204 import evaluator, mechanism as parent_mechanism

OUT = ROOT / "results/analysis/v206/validation200"
INPUT = OUT / "input"
LABEL = "v206_validation200_seed202610030000"
BINS = (PARENT, CHILD)


def records():
    return [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]


def status(state, **extra):
    save(OUT / "status.json", dict(state=state, updated_at=datetime.now().astimezone().isoformat(), **extra))


def verify():
    frozen = json.loads((OUT / "frozen.json").read_text())
    for path, expected in frozen["sha256"].items():
        assert sha(ROOT / path) == expected, path
    return frozen


def prepare():
    assert not OUT.exists(), "already prepared; do not repeat solver runs"
    OUT.mkdir(parents=True)
    before = json.loads((ROOT / "adhoc/v206_audit/preflight.json").read_text())
    for name, expected in before["solver_sha256"].items():
        assert sha(ROOT / f"src/bin/{name}.cpp") == expected
    old_records = records()
    assert not any(r["label"] == LABEL for r in old_records)
    known = {}
    for directory in sorted({r["input_dir"] for r in old_records} | {"tools/in", "tools/validation1"}):
        path = ROOT / directory
        assert path.is_dir(), directory
        known[directory] = {sha(p) for p in evaluator.list_input_files(path)}
    seeds = OUT / "seeds.txt"
    seeds.write_text("".join(f"{202610030000+i}\n" for i in range(200)))
    command = ["sh", "scripts/gen_tools.sh", str(seeds), "--dir", str(INPUT), "--verbose"]
    with (OUT / "input_details.csv").open("w") as stdout, (OUT / "generation.log").open("w") as stderr:
        subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr, check=True)
    inputs = evaluator.list_input_files(INPUT)
    hashes = {sha(p) for p in inputs}
    assert len(inputs) == len(hashes) == 200
    assert not any(hashes & values for values in known.values()), "duplicate existing input"
    archive = OUT / "previous_outputs"
    for name in BINS:
        shutil.copytree(ROOT / "results/out" / name, archive / name)
    save(OUT / "previous_records.json", [
        {**r, "archived_stdout_path": str((archive/r["bin"]/r["case_name"]).relative_to(ROOT))}
        for r in old_records if r["bin"] in BINS])
    paths = [*inputs, seeds, Path(__file__), ROOT / "adhoc/scripts/check_v206.py",
             ROOT / "adhoc/scripts/tune_v204.py", ROOT / "adhoc/scripts/tune_v073.py",
             ROOT / "adhoc/scripts/check_v037_results.py", ROOT / "adhoc/scripts/check_v028_two_orders.py",
             ROOT / "scripts/eval.py", ROOT / "scripts/build_solver.sh",
             ROOT / "tools/src/bin/gen.rs", ROOT / "tools/src/lib.rs"]
    paths.extend(ROOT / f"src/bin/{name}.cpp" for name in BINS)
    paths.extend(p for p in archive.rglob("*") if p.is_file())
    save(OUT / "frozen.json", dict(
        sha256={str(p.relative_to(ROOT)): sha(p) for p in paths},
        bins=BINS, input_dir=str(INPUT.relative_to(ROOT)), label=LABEL,
        generator_command=command, seeds=[202610030000,202610030199],
        known_input_counts={k:len(v) for k,v in known.items()}, duplicate_input_count=0,
        jobs=evaluator.default_jobs(), local=True, local_time_ratio=0.80,
        prior_comparison=json.loads((ROOT / "results/analysis/v206/comparison.json").read_text())))
    (OUT / "preregistration.md").write_bytes((ROOT / "notes/experiments/v206.md").read_bytes())
    status("prepared")
    print("新規200件の入力重複0、両ソース固定、前回の全出力を退避済み。", flush=True)


def analyze():
    frozen = verify()
    selected = [r for r in records() if r["label"] == LABEL and r["input_dir"] == frozen["input_dir"]]
    assert len(selected) == 400
    groups, metrics = {}, {}
    for name in BINS:
        group = sorted((r for r in selected if r["bin"] == name), key=lambda r:r["case_name"])
        assert len(group) == 200 and len({r["run_id"] for r in group}) == 1
        assert {r["case_name"] for r in group} == {p.name for p in INPUT.glob("*.txt")}
        details = []
        for r in group:
            assert r["status"] == "ok" and r["local"]
            case = INPUT / r["case_name"]
            answer = OUT / "outputs" / name / r["case_name"]
            assert sha(answer) == frozen["output_sha256"][str(answer.relative_to(ROOT))]
            log = answer.with_suffix(answer.suffix+".err")
            assert sha(log) == frozen["output_sha256"][str(log.relative_to(ROOT))]
            if name == CHILD:
                m = child_mechanism(str(case), answer)
                c, t = m["counts"], m["times_ms"]
            else:
                c, t = parent_mechanism(case, answer, (5,15))
            assert c["T"] == r["score"] <= 100000
            details.append(dict(case=r["case_name"], counts=c, times_ms=t))
        groups[name] = group
        metrics[name] = {**aggregate(details), "details":details}
    rows = []
    for parent, child in zip(groups[PARENT], groups[CHILD]):
        assert parent["case_name"] == child["case_name"]
        rows.append(dict(case=parent["case_name"], parent_T=parent["score"], T=child["score"],
                         delta_T=child["score"]-parent["score"],
                         parent_elapsed_ms=parent["elapsed"], elapsed_ms=child["elapsed"]))
    ptotal, total = sum(r["parent_T"] for r in rows), sum(r["T"] for r in rows)
    pt, ct = (metrics[name]["average_times_ms"] for name in BINS)
    result = dict(cases=200, input_dir=frozen["input_dir"], label=LABEL,
                  run_ids={name:groups[name][0]["run_id"] for name in BINS},
                  parent_total=ptotal, total=total, parent_average=ptotal/200, average=total/200,
                  delta=total-ptotal, delta_percent=100*(total/ptotal-1),
                  wins=sum(r["delta_T"]<0 for r in rows), draws=sum(r["delta_T"]==0 for r in rows),
                  losses=sum(r["delta_T"]>0 for r in rows),
                  max_elapsed_ms={name:max(r["elapsed"] for r in groups[name]) for name in BINS},
                  attempts={name:metrics[name]["counts"]["lns_attempts"] for name in BINS},
                  lns_excluding_heavy_increase_ms=ct["temporal_lns"]-ct["search_reductions_heavy"]-(pt["temporal_lns"]-pt["search_reductions_heavy"]),
                  all_outputs_verified=True, all_errors_zero=True,
                  improved_on_200=total<ptotal and max(r["elapsed"] for r in selected)<=2000)
    save(OUT / "comparison.json", result)
    save(OUT / "mechanism.json", metrics)
    with (OUT / "cases.csv").open("w", newline="") as f:
        writer=csv.DictWriter(f, fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    print(json.dumps(result,ensure_ascii=False,indent=2), flush=True)
    return result


def run():
    args = argparse.Namespace(bin_name=CHILD, jobs=evaluator.default_jobs(), wait_lock=False)
    try:
        with evaluator.acquire_eval_lock(args, "v206_validation200"):
            prepare()
            inputs = evaluator.list_input_files(INPUT)
            for name in BINS:
                frozen = verify()
                status("running", bin=name)
                args = argparse.Namespace(bin_name=name, jobs=evaluator.default_jobs(), wait_lock=False,
                                          dry_run=False, verbose=False, label=LABEL, no_local=False)
                print(f"評価開始: {name}", flush=True)
                code = evaluator.run_eval_locked(args, inputs, frozen["input_dir"], False, True)
                assert code == 0, f"{name}: eval failed ({code})"
                shutil.copytree(ROOT / "results/out" / name, OUT / "outputs" / name)
                print(f"評価完了: {name}", flush=True)
            frozen = verify()
            frozen["output_sha256"] = {str(p.relative_to(ROOT)):sha(p) for p in (OUT/"outputs").rglob("*") if p.is_file()}
            save(OUT / "frozen.json", frozen)
        analyze()
        status("completed")
    except BaseException as error:
        if OUT.exists():
            status("failed", error=f"{type(error).__name__}: {error}")
        raise


if __name__ == "__main__":
    {"run":run, "analyze":analyze}[sys.argv[1]]()
