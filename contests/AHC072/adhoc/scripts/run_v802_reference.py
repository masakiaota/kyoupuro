#!/usr/bin/env python3
"""validation1の長時間参照を固定条件で採取し、共有または復元する。"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
import csv
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback

import run_v047_long_search as support
from run_v800 import csv_add_once

ROOT = Path(__file__).resolve().parents[2]
BUNDLE = ROOT / "notes/references/validation1_v802"
CASES = [f"{i:04d}.txt" for i in range(500)]
CONFIG = dict(workers=30, v802_seconds=120, v801_round_seconds=60,
              v801_rounds=2, maximum_seconds=6000, seed=80220261005,
              local=True, local_time_ratio=0.80)
SOURCES = ["adhoc/bin/v802.cpp", "adhoc/bin/v800.cpp", "adhoc/bin/v801_teacher.cpp",
           "src/bin/v113_integrated_nn_lns.cpp", "adhoc/scripts/run_v802_reference.py",
           "adhoc/scripts/run_v047_long_search.py", "adhoc/scripts/run_v800.py",
           "scripts/build_solver.sh", "scripts/eval.py", "tools/src/lib.rs",
           "tools/src/bin/vis.rs"]
ENV = os.environ.copy() | support.THREAD_ENV
STOP = threading.Event()


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    support.write_json(path, value)


def sha(path):
    return support.sha256(path)


def state(run, stage, **values):
    save(run / "status.json", dict(stage=stage, updated_at=stamp(), pid=os.getpid(), **values))


def execute(command, log, deadline, inp=None, out=None, timeout=180):
    require(not STOP.is_set() and time.time() < deadline, "中断または実行全体の期限")
    with log.open("wb") as errors:
        stdin = inp.open("rb") if inp else subprocess.DEVNULL
        stdout = out.open("xb") if out else errors
        try:
            began = time.monotonic()
            end = min(deadline, time.time() + timeout)
            process = subprocess.Popen(list(map(str, command)), cwd=ROOT, env=ENV,
                                       stdin=stdin, stdout=stdout, stderr=errors)
            try:
                while process.poll() is None:
                    if STOP.is_set() or time.time() >= end:
                        raise TimeoutError(f"期限または中断: {log}")
                    try:
                        process.wait(timeout=0.25)
                    except subprocess.TimeoutExpired:
                        pass
                require(process.returncode == 0, f"終了コード{process.returncode}: {log}")
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
            return time.monotonic() - began
        finally:
            if inp:
                stdin.close()
            if out:
                stdout.close()


def metrics(inp, plan, scorer):
    value = support.Problem.read(inp).replay(plan.read_text())
    result = subprocess.run([str(scorer), "--no-vis", str(inp), str(plan)],
                            capture_output=True, text=True, check=True, timeout=30)
    require(result.stdout.strip() == f"Score = {value['T']}", f"公式採点不一致: {plan}")
    require(value["E"] == 0, f"全帰巣していない: {plan}")
    return value


def pool(run, stage, tasks, body):
    start = time.monotonic()
    state(run, stage, completed=0, total=len(tasks), workers=CONFIG["workers"])
    with ThreadPoolExecutor(max_workers=CONFIG["workers"]) as executor:
        futures = [executor.submit(body, task) for task in tasks]
        try:
            for count, future in enumerate(as_completed(futures), 1):
                future.result()
                elapsed = time.monotonic() - start
                state(run, stage, completed=count, total=len(tasks), workers=CONFIG["workers"],
                      elapsed_seconds=elapsed, estimated_remaining_seconds=elapsed * (len(tasks)-count)/count)
                if count % 30 == 0 or count == len(tasks):
                    print(f"{stage}: {count}/{len(tasks)}, {elapsed:.1f}秒", flush=True)
        except BaseException:
            STOP.set()
            for future in futures:
                future.cancel()
            raise


def build(run, manifest, deadline):
    for name in ("v802", "v801_teacher", "v113_integrated_nn_lns"):
        for local in ([True] if name.startswith("v113") else [True, False]):
            suffix = "" if local else "_nonlocal"
            command = [ROOT / "scripts/build_solver.sh"] + ([] if local else ["--no-local"]) + [name]
            execute(command, run / f"build_{name}{suffix}.log", deadline, timeout=300)
            shutil.copy2(ROOT / "target/release" / name, run / "frozen" / (name + suffix))
    execute(["cargo", "build", "--release", "--quiet", "-j", "2", "--manifest-path",
             ROOT / "tools/Cargo.toml", "--bin", "vis"], run / "build_vis.log", deadline, timeout=300)
    shutil.copy2(ROOT / "tools/target/release/vis", run / "frozen/vis")
    manifest["build"] = dict(compiler=subprocess.check_output([os.environ.get("CXX", "g++-15"), "--version"], text=True),
        binaries={p.name: sha(p) for p in (run / "frozen").iterdir() if p.is_file()})
    save(run / "manifest.json", manifest)


def prepare(run, deadline):
    require(not (run / "manifest.json").exists(), "既存の採取は再起動しない")
    require(not BUNDLE.exists(), "共有参照は既に存在する。上書きしない")
    require(sorted(p.name for p in (ROOT / "tools/validation1").glob("*.txt")) == CASES,
            "validation1は0000〜0499の500入力が必要")
    a = (ROOT / "adhoc/bin/v802.cpp").read_text().replace("// v802.cpp\n", "// v800.cpp\n", 1)
    require(a == (ROOT / "adhoc/bin/v800.cpp").read_text(), "v802の探索本体がv800と異なる")
    (run / "frozen").mkdir()
    for source in SOURCES:
        dest = run / "frozen" / source
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / source, dest)
    manifest = dict(schema_version=1, experiment="v802", run_id=run.name, created_at=stamp(),
        config=CONFIG, source_sha256={name: sha(ROOT / name) for name in SOURCES},
        host=dict(platform=platform.platform(), cpu=(subprocess.check_output(
            ["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()
            if sys.platform == "darwin" else platform.processor()), logical_cpus=os.cpu_count(),
            memory_bytes=(int(subprocess.check_output(["sysctl", "-n", "hw.memsize"], text=True))
                          if sys.platform == "darwin" else os.sysconf("SC_PAGE_SIZE")*os.sysconf("SC_PHYS_PAGES"))), cases=[])
    save(run / "manifest.json", manifest)
    state(run, "building")
    build(run, manifest, deadline)
    # 同名のscratchは別集合で上書きされ得る。保存記録のスコア一致と
    # 現在の入力からの独立再生が両方成立した出力だけ初期候補にする。
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]
    candidates = {case: {} for case in CASES}
    for row in records:
        if row.get("input_dir") == "tools/validation1" and row.get("status") == "ok":
            if row["case_name"] in candidates:
                candidates[row["case_name"]][row["stdout_path"]] = row
    rejected = []
    state(run, "freezing_saved_seeds")
    for case in CASES:
        folder = run / "cases" / case[:-4]
        folder.mkdir(parents=True)
        inp = folder / "input.txt"
        shutil.copy2(ROOT / "tools/validation1" / case, inp)
        problem = support.Problem.read(inp)
        legal = []
        for rel, row in sorted(candidates[case].items()):
            path = ROOT / rel
            if not path.is_file():
                rejected.append(dict(case=case, source=rel, reason="保存操作列が存在しない"))
                continue
            text = support.canonical_plan(path.read_text())
            try:
                score = problem.replay(text)["T"]
                require(score == row["score"], "保存記録と操作列の手数が異なる")
                legal.append((score, rel, row, text))
            except (ValueError, RuntimeError) as error:
                rejected.append(dict(case=case, source=rel, reason=str(error)))
        require(legal, f"合法な保存初期解がない: {case}")
        T, source, row, text = min(legal, key=lambda value: value[:2])
        support.atomic_text(folder / "saved_seed.txt", text)
        manifest["cases"].append(dict(case_name=case, input_sha256=sha(inp), saved_T=T,
            saved_source=source, saved_source_run_id=row["run_id"], saved_sha256=sha(folder / "saved_seed.txt"),
            features=problem.features()))
    save(run / "rejected_seeds.json", rejected)
    save(run / "manifest.json", manifest)
    return manifest


def baseline(run, item, deadline):
    folder = run / "cases" / item["case_name"][:-4]
    elapsed = execute([run / "frozen/v113_integrated_nn_lns"], folder / "baseline.err", deadline,
                      folder / "input.txt", folder / "baseline.txt", timeout=30)
    result = metrics(folder / "input.txt", folder / "baseline.txt", run / "frozen/vis")
    saved = metrics(folder / "input.txt", folder / "saved_seed.txt", run / "frozen/vis")
    source = "baseline.txt" if result["T"] < saved["T"] else "saved_seed.txt"
    shutil.copy2(folder / source, folder / "initial.txt")
    save(folder / "baseline.json", dict(T=result["T"], saved_T=saved["T"], elapsed_seconds=elapsed,
        initial_T=min(result["T"], saved["T"]), initial_source=source, initial_sha256=sha(folder / "initial.txt")))


def branch(run, task, deadline):
    item, mode = task
    folder = run / "cases" / item["case_name"][:-4]
    base = folder / mode
    base.mkdir()
    initial = folder / "initial.txt"
    rounds = 1 if mode == "v802" else CONFIG["v801_rounds"]
    seconds = CONFIG["v802_seconds"] if mode == "v802" else CONFIG["v801_round_seconds"]
    binary = run / "frozen" / ("v802" if mode == "v802" else "v801_teacher")
    entries = []
    for index in range(rounds):
        part = base / f"round_{index:02d}"
        part.mkdir()
        shutil.copy2(initial, part / "initial.txt")
        seed = int.from_bytes(hashlib.sha256(
            f"{CONFIG['seed']}:{item['case_name']}:{mode}:{index}".encode()).digest()[:8], "big")
        began = stamp()
        elapsed = execute([binary, "--initial-plan", part / "initial.txt", "--output-dir", part / "search",
            "--seconds", seconds, "--seed", seed, "--progress-seconds", "30"],
            part / "stderr.log", deadline, folder / "input.txt", part / "output.txt", timeout=seconds+30)
        result = metrics(folder / "input.txt", part / "output.txt", run / "frozen/vis")
        initial_T = support.Problem.read(folder / "input.txt").replay((part / "initial.txt").read_text())["T"]
        require(result["T"] <= initial_T, "探索結果が初期解より悪化した")
        require(sha(part / "output.txt") == sha(part / "search/best.txt"), "標準出力と保存最良解が異なる")
        events = [json.loads(line) for line in (part / "search/events.jsonl").open()]
        end = events[-1]
        require(events[0]["type"] == "start" and end["type"] == "finish", "探索の開始・終了がない")
        require(end["status"] in ("completed", "time_limit") and end["T"] == result["T"] and end["E"] == 0,
                "探索終了の値が一致しない")
        require(seconds*.98 <= end["elapsed_sec"] <= seconds+30, "探索の時間予算が異なる")
        require(end["attempts"] > 0, "探索機構が発動していない")
        updates = [e["T"] for e in events if e["type"] in ("initial", "improvement")]
        require(updates and updates[0] == initial_T and all(b < a for a, b in zip(updates, updates[1:])),
                "最良更新の単調性が異なる")
        teacher = json.loads((part / "search/teacher_stats.json").read_text()) if mode == "v801" else None
        if teacher is not None:
            require(teacher["nn_calls"] > 0, "v801のNN機構が発動していない")
        entry = dict(round=index, seed=str(seed), requested_seconds=seconds, started_at=began,
            wall_seconds=elapsed, initial_T=initial_T, T=result["T"], initial_sha256=sha(part / "initial.txt"),
            output_sha256=sha(part / "output.txt"), finish=end,
            teacher_mechanism={k: teacher[k] for k in ("nn_calls", "search_reduction_candidates")} if teacher else None)
        save(part / "complete.json", entry)
        entries.append(entry)
        initial = part / "output.txt"
    shutil.copy2(initial, base / "best.txt")
    save(base / "complete.json", dict(T=entries[-1]["T"], rounds=entries,
        wall_seconds=sum(r["wall_seconds"] for r in entries), best_sha256=sha(base / "best.txt")))


def finish(run, manifest):
    state(run, "validating_and_exporting")
    staging = run / "bundle"
    (staging / "outputs").mkdir(parents=True)
    rows, provenance = [], []
    for item in manifest["cases"]:
        case = item["case_name"]
        folder = run / "cases" / case[:-4]
        baseline_value = json.loads((folder / "baseline.json").read_text())
        paths = dict(initial=folder / "initial.txt", v802=folder / "v802/best.txt", v801=folder / "v801/best.txt")
        values = {name: metrics(folder / "input.txt", path, run / "frozen/vis")["T"] for name, path in paths.items()}
        require(all(T <= baseline_value["initial_T"] for T in values.values()), "初期解から悪化した")
        winner = min(values, key=lambda name: (values[name], name))
        dest = staging / "outputs" / case
        shutil.copy2(paths[winner], dest)
        branches = {name: json.loads((folder / name / "complete.json").read_text()) for name in ("v802", "v801")}
        elapsed_ms = round(max(b["wall_seconds"] for b in branches.values()) * 1000)
        rows.append(dict(case_name=case, input_sha256=item["input_sha256"], score=values[winner], T=values[winner], E=0,
            output_sha256=sha(dest), saved_T=item["saved_T"], baseline_T=baseline_value["T"],
            initial_T=baseline_value["initial_T"], v802_T=values["v802"], v801_T=values["v801"],
            winner=winner, elapsed_ms=elapsed_ms))
        provenance.append(dict(**item, baseline=baseline_value, branches=branches, winner=winner))
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=list(rows[0]))
    writer.writeheader()
    writer.writerows(rows)
    support.atomic_text(staging / "scores.csv", output.getvalue())
    result = dict(cases=len(rows), total=sum(r["T"] for r in rows),
        initial_total=sum(r["initial_T"] for r in rows), baseline_total=sum(r["baseline_T"] for r in rows),
        saved_total=sum(r["saved_T"] for r in rows), improved_cases=sum(r["T"] < r["initial_T"] for r in rows),
        winner_counts={name: sum(r["winner"] == name for r in rows) for name in ("initial", "v802", "v801")},
        round_count=sum(len(b["rounds"]) for p in provenance for b in p["branches"].values()))
    portable = dict(schema_version=1, bin="v802", input_dir="tools/validation1", run_id=manifest["run_id"],
        executed_at=stamp(), label="固定長時間参照 validation1: v800 120秒 + v801 60秒×2, j30",
        score_kind="best_known_feasible_upper_bound", optimality_proven=False,
        scores_sha256=sha(staging / "scores.csv"), config=CONFIG, host=manifest["host"],
        source_sha256=manifest["source_sha256"], build=manifest["build"], result=result, provenance=provenance)
    save(staging / "manifest.json", portable)
    require(not BUNDLE.exists(), "共有参照の上書きを拒否")
    BUNDLE.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(staging, BUNDLE)
    install(BUNDLE, run / "frozen/vis")
    save(run / "result.json", result)
    state(run, "completed", result=result, bundle=str(BUNDLE.relative_to(ROOT)))
    print(json.dumps(result, ensure_ascii=False), flush=True)


def install(bundle, scorer):
    manifest = json.loads((bundle / "manifest.json").read_text())
    require(sha(bundle / "scores.csv") == manifest["scores_sha256"], "CSVのハッシュが異なる")
    rows = list(csv.DictReader((bundle / "scores.csv").open()))
    require([r["case_name"] for r in rows] == CASES, "参照が500件揃っていない")
    require(sorted(p.name for p in (bundle / "outputs").glob("*.txt")) == CASES, "操作列が500件揃っていない")
    spec = importlib.util.spec_from_file_location("v802_eval", ROOT / "scripts/eval.py")
    evaluator = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = evaluator
    spec.loader.exec_module(evaluator)
    results = []
    for row in rows:
        name = row["case_name"]
        inp, output = ROOT / "tools/validation1" / name, bundle / "outputs" / name
        require(sha(inp) == row["input_sha256"] and sha(output) == row["output_sha256"], f"ハッシュ不一致: {name}")
        value = metrics(inp, output, scorer)
        require(value["T"] == int(row["T"]) == int(row["score"]) and int(row["E"]) == 0,
                f"スコア不一致: {name}")
        results.append(evaluator.CaseResult(name, "ok", value["T"], int(row["elapsed_ms"]), f"results/out/v802/{name}"))
    records = evaluator.make_records(results, manifest["run_id"], manifest["executed_at"], "v802",
                                     manifest["label"], "tools/validation1", True)
    for record, row in zip(records, rows):
        record.update(reference=True, input_sha256=row["input_sha256"], output_sha256=row["output_sha256"],
                      reference_bundle=str(bundle.relative_to(ROOT)), optimality_proven=False)
    record_file = ROOT / "results/eval_records.jsonl"
    original = record_file.read_text() if record_file.exists() else ""
    same = [json.loads(line) for line in original.splitlines() if line.strip()
            and json.loads(line).get("run_id") == manifest["run_id"]]
    require(not same or same == records, "同じrun_idの記録が一致しない")
    # 全検証後に公開する。viewerが読むJSONLは最後に全500行まとめて書く。
    for row in rows:
        support.atomic_text(ROOT / "results/out/v802" / row["case_name"],
                            (bundle / "outputs" / row["case_name"]).read_text())
    avg, total, low, high, avg_ms, max_ms = evaluator.summarize(results)
    csv_add_once(ROOT / "results/score_summary.csv", evaluator.SUMMARY_HEADER,
        ["v802", avg, total, low, high, avg_ms, max_ms, "tools/validation1", 500,
         manifest["label"], manifest["executed_at"]], [0, 9, 10])
    if not same:
        block = "".join(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n" for row in records)
        support.atomic_text(record_file, original + ("\n" if original and not original.endswith("\n") else "") + block)
    print(f"v802の固定参照500件を評価記録へ登録した: {total}手", flush=True)


def compare(bundle):
    manifest = json.loads((bundle / "manifest.json").read_text())
    require(sha(bundle / "scores.csv") == manifest["scores_sha256"], "CSVのハッシュが異なる")
    rows = list(csv.DictReader((bundle / "scores.csv").open()))
    require([r["case_name"] for r in rows] == CASES, "参照のケース集合が異なる")
    for row in rows:
        require(sha(ROOT / "tools/validation1" / row["case_name"]) == row["input_sha256"], "入力ハッシュが異なる")
    ref = {r["case_name"]: int(r["score"]) for r in rows}
    groups = {}
    for line in (ROOT / "results/eval_records.jsonl").open():
        row = json.loads(line)
        if row.get("input_dir") == "tools/validation1":
            groups.setdefault(row["run_id"], []).append(row)
    scores = []
    for run_id, records in groups.items():
        values = {r["case_name"]: r["score"] for r in records}
        if len(records) != 500 or values.keys() != ref.keys() or not all(r["status"] == "ok" for r in records):
            continue
        total = sum(values.values())
        # ケースごとの丸めは公式式と同じ。固定参照より短い候補もそのまま表示する。
        relative = sum((2*10**9*ref[k] + values[k]) // (2*values[k]) for k in ref) / 500
        scores.append(dict(bin=records[0]["bin"], total=total, mean=total/500,
            relative_avg_1e9=relative, relative_avg_100=relative/10**7, run_id=run_id,
            label=records[0].get("label", "")))
    return sorted(scores, key=lambda row: -row["relative_avg_1e9"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--run", type=Path, help="新規採取の保存先。既存採取は再実行しない")
    group.add_argument("--install", action="store_true", help="共有参照を検証して評価画面へ復元する。solver実行なし")
    group.add_argument("--compare", action="store_true", help="保存評価を固定参照で集計する。solver実行なし")
    args = parser.parse_args()
    if args.compare:
        print(json.dumps(compare(BUNDLE), ensure_ascii=False, indent=2))
        return
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda *_: STOP.set())
    with support.evaluation_lock():
        if args.install:
            subprocess.run(["cargo", "build", "--release", "--quiet", "-j", "2", "--manifest-path",
                            str(ROOT / "tools/Cargo.toml"), "--bin", "vis"], check=True)
            install(BUNDLE, ROOT / "tools/target/release/vis")
            return
        run = args.run.resolve()
        run.mkdir(parents=True, exist_ok=True)
        deadline = time.time() + CONFIG["maximum_seconds"]
        try:
            manifest = prepare(run, deadline)
            warm = run / "warmup"
            warm.mkdir()
            inp = run / "cases/0000/input.txt"
            execute([run / "frozen/v113_integrated_nn_lns"], warm / "stderr.log", deadline,
                    inp, warm / "output.txt", timeout=30)
            metrics(inp, warm / "output.txt", run / "frozen/vis")
            pool(run, "baselines", manifest["cases"], lambda item: baseline(run, item, deadline))
            tasks = [(item, mode) for item in manifest["cases"] for mode in ("v802", "v801")]
            pool(run, "long_search", tasks, lambda task: branch(run, task, deadline))
            finish(run, manifest)
        except BaseException as error:
            STOP.set()
            save(run / "failure.json", dict(error=repr(error), at=stamp(), traceback=traceback.format_exc()))
            state(run, "failed", error=repr(error))
            raise


if __name__ == "__main__":
    main()
