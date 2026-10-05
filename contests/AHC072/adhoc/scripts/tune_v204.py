#!/usr/bin/env python3
"""固定した9条件を一度ずつ比較し、選択と確認評価を生成AIなしで完了する。"""
from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import replace
from datetime import datetime
import difflib
import json
import os
from pathlib import Path
import random
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import traceback

from tune_v073 import (FLAGS, MODES, configure_environment, digest, evaluator,
                       execute, now, parallel_cases, write_json)
from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT / "src/bin/v203_floor_nn_initial_race.cpp"
SOURCE = ROOT / "src/bin/v204_initial_race_tuned.cpp"
AUDIT = ROOT / "adhoc/v204_audit"
RUN = ROOT / "results/tuning/v204"
NOTE = ROOT / "notes/experiments/v204.md"
PARENT_SHA = "484dc789e961281038fa42ea79bfd12ad385ebdf3b68dcfff9f9978db157d3d4"
PARENT_RUN = "20261003T024151+0900_v203_floor_nn_initial_race_e82872"
CONFIGS = [(4, 15)] + [(n, p) for n in (3, 4, 5) for p in (10, 15, 20) if (n, p) != (4, 15)]
ORDER = list(range(1, len(CONFIGS)))
random.Random(204).shuffle(ORDER)
ORDER.insert(0, 0)


def name(config):
    return f"v204_c{config[0]}_p{config[1]}"


def edits(config, filename):
    count, percent = config
    assert config in CONFIGS
    return [
        ("// v203_floor_nn_initial_race.cpp", "// " + filename),
        ("entries.size()==4", f"entries.size()=={count}"),
        ("entries.size()>4", f"entries.size()>{count}"),
        ("// 最初の2段階に各15%を使い、最短候補の継続探索へ70%を残す。",
         f"// 最初の2段階に各{percent}%を使い、最短候補の継続探索へ{100-2*percent}%を残す。"),
        ("budget*(0.15*(round+1))", f"budget*({percent/100:.2f}*(round+1))"),
    ]


def source_text(config, filename):
    assert digest(PARENT) == PARENT_SHA
    text = PARENT.read_text()
    for old, new in edits(config, filename):
        assert text.count(old) == 1, old
        text = text.replace(old, new, 1)
    return text


def rank_key(row):
    count, percent = CONFIGS[row["config_id"]]
    return row["total_sum"], abs(count-4)+abs(percent-15)//5, count, percent


def choose(rows):
    return min((r for r in rows if r["max_elapsed_ms"] <= 2000), key=rank_key)


def normalize(text, path):
    text = text.replace(str(path), "solver.cpp").replace('"'+path.name+'"', '"solver.cpp"')
    return re.sub(r'("solver\.cpp",\s*)\d+(,)', r"\g<1>0\2", text)


def prepare():
    """solverを実行せず、全条件と判定手順を確定する。"""
    configure_environment()
    assert not (AUDIT / "preflight.json").exists() and not SOURCE.exists()
    AUDIT.mkdir(parents=True, exist_ok=True)
    RUN.mkdir(parents=True, exist_ok=True)
    SOURCE.write_text(source_text(CONFIGS[0], SOURCE.name))
    changes = {}
    for config in CONFIGS:
        path = ROOT / "adhoc/bin" / (name(config)+".cpp")
        assert not path.exists()
        path.write_text(source_text(config, path.name))
        changes[path.name] = edits(config, path.name)
        (AUDIT / (path.stem+".diff")).write_text("".join(difflib.unified_diff(
            PARENT.read_text().splitlines(True), path.read_text().splitlines(True),
            fromfile=PARENT.name, tofile=path.name)))
    write_json(AUDIT / "registered_changes.json", changes)
    compiler = os.environ.get("CXX", "g++-15")
    parent_expanded = {}
    for mode, flags in MODES.items():
        text = subprocess.check_output([compiler, *FLAGS, *flags, "-E", "-P", str(PARENT)], text=True)
        (AUDIT / ("parent_"+mode+".ii")).write_text(text)
        parent_expanded[mode] = normalize(text, PARENT)

    def build(item):
        config, mode = item
        path = ROOT / "adhoc/bin" / (name(config)+".cpp")
        binary = AUDIT / (path.stem+"_"+mode)
        command = [compiler, *FLAGS, *MODES[mode], str(path), "-o", str(binary)]
        execute(command, binary.with_suffix(".build.log"))
        expanded = subprocess.check_output([compiler, *FLAGS, *MODES[mode], "-E", "-P", str(path)], text=True)
        restored = expanded
        for old, new in reversed(edits(config, path.name)):
            if old.startswith("//"):
                continue
            assert restored.count(new) == 1, new
            restored = restored.replace(new, old, 1)
        assert normalize(restored, path) == parent_expanded[mode], (config, mode)
        return dict(config=list(config), mode=mode, source=str(path.relative_to(ROOT)),
                    source_sha256=digest(path), binary=str(binary.relative_to(ROOT)),
                    binary_sha256=digest(binary), registered_difference_only=True)

    builds = []
    args = argparse.Namespace(bin_name="v204_prepare", jobs=2, wait_lock=False)
    with evaluator.acquire_eval_lock(args, "no_solver_execution"):
        parallel_cases([(c, m) for c in CONFIGS for m in MODES], build, builds.append)
        evaluator.build_tool("gen")
        evaluator.build_tool(evaluator.SCORER_BIN_NAME)
    seeds = sorted({secrets.randbits(62) | (1 << 62) for _ in range(100)})
    assert len(seeds) == 100
    (RUN / "seeds.txt").write_text("".join(f"{s}\n" for s in seeds))
    execute([str(evaluator.TOOLS_BIN_DIR / "gen"), str(RUN / "seeds.txt"), "--dir", str(RUN / "input")], RUN / "generation.log")
    inputs = sorted((RUN / "input").glob("*.txt"))
    previous = {digest(p) for p in (ROOT / "tools").glob("*/*.txt")}
    for manifest in (ROOT / "results/tuning").glob("*/input_manifest.json"):
        if manifest.parent != RUN:
            previous.update(c["sha256"] for c in json.loads(manifest.read_text())["cases"])
    hashes = [digest(p) for p in inputs]
    assert len(inputs) == len(set(hashes)) == 100 and not set(hashes) & previous
    write_json(RUN / "input_manifest.json", dict(cases=[dict(case=p.name, seed=s, sha256=h)
               for p, s, h in zip(inputs, seeds, hashes)], reusable_for_future_evaluation=False))
    parent_records = [json.loads(line) for line in evaluator.RECORDS_JSONL.open()
                      if f'"{PARENT_RUN}"' in line]
    assert len(parent_records) == 100 and all(r["status"] == "ok" for r in parent_records)
    write_json(RUN / "parent_records.json", parent_records)
    write_json(RUN / "plan.json", dict(configurations=CONFIGS, order=ORDER, jobs=2, warmup_cases=1,
               tuning_cases=100, confirmation_cases=100, local_time_ratio=0.8, parent_run=PARENT_RUN))
    shutil.copy2(NOTE, RUN / "preregistration.md")
    write_json(AUDIT / "preflight.json", dict(passed=True, solver_executions=0, builds=builds))
    fixed = [PARENT, Path(__file__), NOTE, RUN / "preregistration.md", RUN / "plan.json",
             RUN / "parent_records.json", RUN / "input_manifest.json", RUN / "seeds.txt",
             AUDIT / "preflight.json", AUDIT / "registered_changes.json",
             ROOT / "scripts/eval.py", ROOT / "adhoc/scripts/tune_v073.py",
             ROOT / "adhoc/scripts/check_v037_results.py", ROOT / "adhoc/scripts/check_v028_two_orders.py",
             ROOT / "adhoc/scripts/audit_v057.py", *inputs, *(ROOT / "tools/in").glob("*.txt"),
             evaluator.TOOLS_BIN_DIR / "vis"]
    for b in builds:
        fixed.extend([ROOT / b["source"], ROOT / b["binary"]])
    write_json(RUN / "frozen.json", {str(p.relative_to(ROOT)): digest(p) for p in fixed})
    print("9条件×LOCAL・非LOCAL: ビルドと完全な前処理照合に成功。solver実行0回。", flush=True)


def verify_frozen():
    for rel, expected in json.loads((RUN / "frozen.json").read_text()).items():
        assert digest(ROOT / rel) == expected, rel


def mechanism(input_path, answer, config):
    T = verify_output(str(input_path), answer)
    c, times, diagnostics = log_values(answer.with_suffix(answer.suffix+".err"))
    assert T == c["T"] == c["final_ops"] == c["validated_moves"] and c["E"] == 0
    assert not diagnostics and not any(c[key] for key in ERRORS)
    assert c["state_pool_free_at_end"] == c["state_slots"] == 4
    count = c["race_seed_count"]
    assert 1 <= count <= config[0]
    initial = {i: c[f"race_seed_{i}_initial_ops"] for i in range(count)}
    assert list(initial.values()) == sorted(initial.values())
    latest, active, attempts, resumes = initial.copy(), list(range(count)), 0, 0
    for stage in range(2):
        if len(active) == 1:
            break
        assert c[f"race_round_{stage}_grown"] == len(active)
        if stage:
            resumes += len(active)
        for i in active:
            n = c[f"race_round_{stage}_seed_{i}_ops"]
            assert n <= latest[i]
            latest[i] = n
            attempts += c[f"race_round_{stage}_seed_{i}_attempts"]
        active.sort(key=lambda i: (latest[i], i))
        active = active[:(len(active)+1)//2 if stage == 0 else 1]
        assert c[f"race_round_{stage}_survivors"] == len(active)
    winner = active[0]
    assert c["race_winner_initial_rank"] == winner
    assert c["race_winner_changed"] == int(winner != 0)
    assert c["race_selection_saved"] == latest[0]-latest[winner]
    assert c["race_round_2_grown"] == 1
    assert c[f"race_round_2_seed_{winner}_ops"] <= latest[winner]
    latest[winner] = c[f"race_round_2_seed_{winner}_ops"]
    attempts += c[f"race_round_2_seed_{winner}_attempts"]
    resumes += count > 1
    assert c["race_resumes"] == resumes and attempts == c["lns_attempts"]
    assert all(latest[i] == c[f"race_seed_{i}_final_ops"] for i in range(count))
    assert T <= min(latest.values()) <= initial[0]
    enabled = int(c["floor_cells"] >= 193)
    assert c["selector_nn_enabled"] == enabled and (c["nn_calls"] > 0) == bool(enabled)
    return c, times


def status(state, stage, **extra):
    write_json(RUN / "status.json", dict(status=state, stage=stage, pid=os.getpid(), updated_at=now(), **extra))
    print(f"{now()} {state}: {stage}", flush=True)


def evaluate(config_id, confirmation=False):
    verify_frozen()
    config = CONFIGS[config_id]
    bin_name = SOURCE.stem if confirmation else name(config)
    input_dir = ROOT / "tools/in" if confirmation else RUN / "input"
    inputs = sorted(input_dir.glob("*.txt"))
    binary = AUDIT / (name(config)+"_local")
    output = ROOT / "results/out" / bin_name
    output.mkdir(parents=True, exist_ok=False)
    scorer = evaluator.TOOLS_BIN_DIR / "vis"
    stage = "confirmation" if confirmation else f"tuning_{config_id}"
    status("running", stage, config=list(config), completed_cases=0)
    # 通常評価と同じ先頭1件のウォームアップを使い、その出力と計数は残さない。
    with tempfile.TemporaryDirectory(prefix=".warmup_", dir=output) as temp:
        result = evaluator.run_case(inputs[0], binary, scorer, Path(temp), False)
        assert result.status == "ok"
        counts, _ = mechanism(inputs[0], ROOT / result.stdout_path, config)
        assert counts["race_seed_count"] > 1 and counts["race_resumes"] > 0
    measured, counts_total, seed_counts = [], Counter(), Counter()
    time_total = Counter()
    executed = datetime.now().astimezone()
    label = f"v204_{stage}_c{config[0]}_p{config[1]}"
    run_id = evaluator.make_run_id(executed, bin_name)
    input_names = {p.name: p for p in inputs}

    def accept(result):
        counts, times = {}, {}
        if result.status == "ok":
            try:
                counts, times = mechanism(input_names[result.case_name], ROOT / result.stdout_path, config)
            except Exception:
                result = replace(result, status="mechanism_fail", score=None)
                (RUN / "validation_error.txt").write_text(traceback.format_exc())
        records = evaluator.make_records([result], run_id, executed.isoformat(timespec="seconds"),
                  bin_name, label, evaluator.normalize_dir(input_dir), True)
        evaluator.append_jsonl(evaluator.RECORDS_JSONL, records)
        evaluator.append_jsonl(RUN / "cases.jsonl", [dict(records[0], config_id=config_id)])
        measured.append(result)
        assert result.status == "ok", (stage, result.case_name, result.status)
        counts_total.update(counts)
        time_total.update(times)
        seed_counts[counts["race_seed_count"]] += 1
        if len(measured) % 10 == 0:
            status("running", stage, config=list(config), completed_cases=len(measured))

    parallel_cases(inputs, lambda p: evaluator.run_case(p, binary, scorer, output, False), accept)
    assert len(measured) == 100 and counts_total["race_resumes"] > 0
    assert counts_total["lns_attempts"] > 0
    # 上限まで候補を保持する実行がなければ、候補数変更を比較したことにならない。
    assert seed_counts[config[0]] > 0, (config, seed_counts)
    verify_frozen()
    metrics = evaluator.summarize(measured)
    evaluator.append_csv_row(evaluator.SUMMARY_CSV, [bin_name, *metrics,
        evaluator.normalize_dir(input_dir), len(measured), label, executed.isoformat(timespec="seconds")])
    if confirmation:
        header = evaluator.compute_detail_header()
        evaluator.ensure_csv_header(evaluator.DETAIL_CSV, header)
        by_case = {r.case_name: r.score for r in measured}
        evaluator.append_csv_row(evaluator.DETAIL_CSV, [bin_name, metrics[0], metrics[-1],
            *[by_case[c] for c in header[3:-2]], label, executed.isoformat(timespec="seconds")])
    row = dict(config_id=config_id, config=list(config), run_id=run_id,
               total_sum=metrics[1], total_avg=metrics[1]/100, max_elapsed_ms=metrics[-1],
               seed_counts=dict(seed_counts), winner_changed=counts_total["race_winner_changed"],
               race_resumes=counts_total["race_resumes"], lns_attempts=counts_total["lns_attempts"],
               stage_times_ms={str(i):time_total[f"race_round_{i}"] for i in range(3)},
               scores={r.case_name:r.score for r in measured})
    write_json(RUN / (stage+".json"), row)
    return row


def production_diagnostic():
    records = []
    for config in ((3, 10), (5, 20)):
        out = AUDIT / (name(config)+"_production_diagnostic.txt")
        case = sorted((RUN / "input").glob("*.txt"))[0]
        with case.open() as stdin, out.open("w") as stdout, out.with_suffix(".txt.err").open("w") as stderr:
            subprocess.run([str(AUDIT / (name(config)+"_production"))], stdin=stdin,
                           stdout=stdout, stderr=stderr, check=True, timeout=15)
        T = verify_output(str(case), out)
        assert "diagnostic:" not in out.with_suffix(".txt.err").read_text()
        records.append(dict(config=list(config), T=T, legal=True))
    write_json(AUDIT / "production_diagnostic.json", records)


def worker():
    configure_environment()
    assert not (RUN / "started.json").exists(), "同一調整の再起動は認めない"
    write_json(RUN / "started.json", dict(pid=os.getpid(), started_at=now()))
    awake = None
    try:
        if sys.platform == "darwin":
            awake = subprocess.Popen(["caffeinate", "-i", "-w", str(os.getpid())])
        args = argparse.Namespace(bin_name=SOURCE.stem, jobs=2, wait_lock=False)
        with evaluator.acquire_eval_lock(args, "v204_tuning_and_confirmation"):
            verify_frozen()
            evaluator.ensure_csv_header(evaluator.SUMMARY_CSV, evaluator.SUMMARY_HEADER)
            production_diagnostic()
            rows = []
            for config_id in ORDER:
                rows.append(evaluate(config_id))
                write_json(RUN / "trials.json", rows)
            best = choose(rows)
            baseline = next(r for r in rows if r["config_id"] == 0)
            # 選択前に存在した候補の文字列のみから最終提出用ファイルを作る。
            selected = source_text(CONFIGS[best["config_id"]], SOURCE.name)
            SOURCE.write_text(selected)
            expected = (ROOT / "adhoc/bin" / (name(CONFIGS[best["config_id"]])+".cpp")).read_text()
            assert selected.split("\n", 1)[1] == expected.split("\n", 1)[1]
            confirmation = evaluate(best["config_id"], True) if best["config_id"] != 0 else None
            parents = json.loads((RUN / "parent_records.json").read_text())
            parent_sum = sum(r["score"] for r in parents)
            delta = confirmation["total_sum"]-parent_sum if confirmation else None
            result = dict(best=best, baseline=baseline, tuning_delta=best["total_sum"]-baseline["total_sum"],
                          confirmation=confirmation, confirmation_delta=delta, parent_total_sum=parent_sum,
                          adopted=bool(confirmation and delta < 0 and confirmation["max_elapsed_ms"] <= 2000),
                          source=str(SOURCE.relative_to(ROOT)), source_sha256=digest(SOURCE))
            write_json(RUN / "result.json", result)
            status("completed", "all_finished", result="results/tuning/v204/result.json")
    except BaseException:
        status("failed", "stopped", error=traceback.format_exc())
        raise
    finally:
        if awake is not None:
            awake.terminate()
            awake.wait()


def launch():
    verify_frozen()
    assert not (RUN / "launch.json").exists() and not (RUN / "started.json").exists()
    with (RUN / "run.log").open("x") as log:
        child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "worker"],
                                 cwd=ROOT, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                                 start_new_session=True)
    result = dict(pid=child.pid, started_at=now(), expected_minutes=15,
                  status_path=str(RUN / "status.json"), log_path=str(RUN / "run.log"))
    write_json(RUN / "launch.json", result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    {"prepare": prepare, "launch": launch, "worker": worker}[sys.argv[1]]()
