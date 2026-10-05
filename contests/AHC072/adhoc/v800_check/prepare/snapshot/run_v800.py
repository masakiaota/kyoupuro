#!/usr/bin/env python3
"""90件を各5分、探索2プロセスで処理し、既存10件と合わせてv800を登録する。"""

from __future__ import annotations

import argparse
import csv
from datetime import datetime
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import uuid

import run_v047_long_search as search

ROOT = Path(__file__).resolve().parents[2]
CASES = [f"{i:04d}" for i in range(100)]
REUSE_CASES = set(search.DEFAULT_CASES)
LABEL = "手元最良参照: 90件×300秒、探索2並列、既存10件再利用"
SNAPSHOTS = ["adhoc/bin/v800.cpp", "adhoc/scripts/run_v800.py",
             "adhoc/scripts/run_v047_long_search.py", "scripts/eval.py",
             "adhoc/bin/v047_long_search.cpp", "src/bin/v050_joint_towers.cpp",
             "notes/experiments/v800.md", "tools/src/bin/vis.rs", "tools/src/lib.rs"]


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def read_json(path):
    return json.loads(path.read_text())


def choose_reuse(explicit):
    candidates = [explicit.resolve()] if explicit else sorted(
        (ROOT / "results/long_search/v047").glob("*"), reverse=True)
    for path in candidates:
        if not (path / "status.json").is_file() or read_json(path / "status.json").get("status") != "completed":
            continue
        manifest = read_json(path / "manifest.json")
        by_case = {c["case"]: c for c in manifest["cases"]}
        if not REUSE_CASES <= by_case.keys():
            continue
        for case_id in REUSE_CASES:
            case = by_case[case_id]
            expected = search.sha256(ROOT / "tools/in" / f"{case_id}.txt")
            if case["input_sha256"] != expected or search.sha256(path / case["input"]) != expected:
                raise ValueError(f"再利用元と現在の入力が異なる: {case_id}")
            for mode in search.MODES:
                if read_json(path / "cases" / case_id / mode / "status.json")["status"] != "completed":
                    raise ValueError(f"再利用元が未完了: {case_id}/{mode}")
        return path
    raise ValueError("既存10ケースが正常完了したv047記録が見つからない。--reuse-runで指定できる")


def prepare(args, run):
    reuse = choose_reuse(args.reuse_run)
    actual = sorted(p.name for p in (ROOT / "tools/in").iterdir() if p.is_file())
    if actual != [c + ".txt" for c in CASES]:
        raise ValueError("tools/inは0000.txt〜0099.txtの100入力である必要がある")
    run.mkdir(parents=True, exist_ok=False)
    (run / "snapshot").mkdir()
    executed_at = datetime.now().astimezone().isoformat(timespec="seconds")
    manifest = dict(schema_version=1, experiment="v800", run_id=run.name + "_v800",
                    executed_at=executed_at, label=LABEL, created_at=search.utc_now(),
                    reuse_run=str(reuse.relative_to(ROOT)) if reuse.is_relative_to(ROOT) else str(reuse),
                    config=dict(minutes_per_case=5.0, workers=2, restart_seconds=60,
                                progress_seconds=10, seed=args.seed, thread_env=search.THREAD_ENV),
                    files={}, cases=[])
    for name in SNAPSHOTS:
        source = ROOT / name
        destination = run / "snapshot" / source.name
        shutil.copy2(source, destination)
        manifest["files"][str(destination.relative_to(run))] = dict(source=name, sha256=search.sha256(destination))
    directories = sorted(p for p in (ROOT / "results/out").iterdir()
                         if p.is_dir() and re.match(r"^v\d{3}(?:_|$)", p.name) and p.name != "v000_template")
    rejected = []
    for case_id in CASES:
        base = run / "cases" / case_id
        (base / "seeds").mkdir(parents=True)
        inp = ROOT / "tools/in" / (case_id + ".txt")
        shutil.copy2(inp, base / "input.txt")
        problem = search.Problem.read(inp)
        candidates, seen = [], set()
        sources = [p / (case_id + ".txt") for p in directories]
        reused = case_id in REUSE_CASES
        if reused:
            sources.append(reuse / "cases" / case_id / "best.txt")
            if not sources[-1].is_file():
                raise ValueError(f"再利用する長時間最良解がない: {case_id}")
        for source in sources:
            if not source.is_file():
                continue
            try:
                text = search.canonical_plan(source.read_text())
                digest = hashlib.sha256(text.encode()).hexdigest()
                if digest in seen:
                    continue
                metrics = problem.replay(text)
                seen.add(digest)
                candidates.append(dict(source=str(source.relative_to(ROOT)) if source.is_relative_to(ROOT) else str(source),
                                       source_sha256=search.sha256(source), plan_sha256=digest, metrics=metrics, text=text))
            except (ValueError, OSError) as exc:
                rejected.append(dict(case=case_id, source=str(source), error=str(exc)))
                if reused and source == sources[-1]:
                    raise ValueError(f"再利用する長時間最良解が不正: {case_id}") from exc
        candidates.sort(key=lambda c: (c["metrics"]["T"], c["source"]))
        if not candidates:
            raise ValueError(f"合法な保存初期解が見つからない: {case_id}")
        seeds = []
        for index, candidate in enumerate(candidates[:8]):
            path = base / "seeds" / f"{index:02d}.txt"
            path.write_text(candidate["text"])
            seeds.append({k: v for k, v in candidate.items() if k != "text"} | {"path": str(path.relative_to(run))})
        search.atomic_text(base / "best.txt", candidates[0]["text"])
        elapsed_ms = 0
        if reused:
            elapsed_ms = round(1000 * max(read_json(reuse / "cases" / case_id / mode / "status.json")["elapsed_sec"]
                                          for mode in search.MODES))
        case = dict(case=case_id, input=str((base / "input.txt").relative_to(run)), input_sha256=search.sha256(inp),
                    reference_T=seeds[0]["metrics"]["T"], seeds=seeds, features=problem.features(),
                    reused=reused, reused_elapsed_ms=elapsed_ms)
        manifest["cases"].append(case)
        search.write_json(base / "status.json", dict(status="reused" if reused else "prepared", T=case["reference_T"],
                          elapsed_ms=elapsed_ms, new_elapsed_ms=0, best_source=seeds[0]["source"],
                          best_sha256=search.sha256(base / "best.txt"), attempts=[]))
    search.write_json(run / "manifest.json", manifest)
    search.write_json(run / "rejected_seeds.json", rejected)
    search.write_json(run / "status.json", dict(status="prepared", updated_at=search.utc_now()))
    print(f"保存先: {run}\n準備完了: 探索90件×5分、再利用10件、同時探索2プロセス", flush=True)
    return manifest


def verify_snapshot(run, manifest):
    if manifest["config"]["minutes_per_case"] != 5 or manifest["config"]["workers"] != 2:
        raise ValueError("v800の探索条件は1ケース5分、同時2プロセスに固定")
    if [c["case"] for c in manifest["cases"]] != CASES:
        raise ValueError("0000〜0099の100ケースが揃っていない")
    if {c["case"] for c in manifest["cases"] if c["reused"]} != REUSE_CASES:
        raise ValueError("再利用ケースが準備時の10件と異なる")
    for name, record in manifest["files"].items():
        if search.sha256(run / name) != record["sha256"]:
            raise ValueError(f"保存した実行資料が変更されている: {name}")
    for name in ("run_v800.py", "run_v047_long_search.py"):
        if search.sha256(ROOT / "adhoc/scripts" / name) != manifest["files"]["snapshot/" + name]["sha256"]:
            raise ValueError(f"実行管理コードが準備時から変わっている: {name}")
    for case in manifest["cases"]:
        for inp in (run / case["input"], ROOT / "tools/in" / (case["case"] + ".txt")):
            if search.sha256(inp) != case["input_sha256"]:
                raise ValueError(f"入力が準備時から変わっている: {inp}")
        for seed in case["seeds"]:
            if search.sha256(run / seed["path"]) != seed["plan_sha256"]:
                raise ValueError(f"保存初期解が変更されている: {seed['path']}")


def build(run, manifest):
    binary, scorer = run / "snapshot/v800", run / "snapshot/vis"
    if "build" in manifest:
        for path, key in ((binary, "binary_sha256"), (scorer, "scorer_sha256")):
            if search.sha256(path) != manifest["build"][key]:
                raise ValueError(f"実行ファイルが変更されている: {path}")
        return binary, scorer
    env = os.environ.copy() | search.THREAD_ENV | {"CARGO_BUILD_JOBS": "2"}
    if sys.platform == "darwin":
        env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
        env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
    compiler = shutil.which(os.environ.get("CXX", "g++-15"))
    if not compiler:
        raise ValueError("GCC 15が見つからない。CXXで指定できる")
    command = [compiler, "-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
               "-ftrivial-auto-var-init=zero", "-fopenmp", "-DLOCAL", str(run / "snapshot/v800.cpp"), "-o", str(binary)]
    with (run / "build.log").open("w") as log:
        subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run(["cargo", "build", "--release", "--quiet", "-j", "2", "--manifest-path",
                        str(ROOT / "tools/Cargo.toml"), "--bin", "vis"], cwd=ROOT, env=env,
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    shutil.copy2(ROOT / "tools/target/release/vis", scorer)
    manifest["build"] = dict(command=command, compiler_version=subprocess.check_output([compiler, "--version"], text=True),
                              binary_sha256=search.sha256(binary), scorer_sha256=search.sha256(scorer), local=True)
    search.write_json(run / "manifest.json", manifest)
    return binary, scorer


def reconcile_attempts(run, case):
    """A kill between saving an attempt and its case checkpoint must not lose its best plan."""
    base = run / "cases" / case["case"]
    state = read_json(base / "status.json")
    problem = search.Problem.read(run / case["input"])
    best_T = problem.replay((base / "best.txt").read_text())["T"]
    new_ms, attempts = 0, []
    completed = case["reused"] or state["status"] == "completed"
    for attempt in sorted((base / "attempts").glob("attempt_*")):
        if not (attempt / "manifest.json").exists():
            raise ValueError(f"準備中に停止した探索記録: {attempt}")
        status_file = attempt / "status.json"
        status = read_json(status_file).get("status", "interrupted") if status_file.exists() else "interrupted"
        elapsed = 0
        for mode in search.MODES:
            worker = attempt / "cases" / case["case"] / mode
            if (worker / "status.json").exists():
                elapsed = max(elapsed, read_json(worker / "status.json").get("elapsed_sec", 0))
            if (worker / "best.txt").exists():
                text = (worker / "best.txt").read_text()
                T = problem.replay(text)["T"]
                if T < best_T:
                    search.atomic_text(base / "best.txt", text)
                    state["best_source"] = str((worker / "best.txt").relative_to(run))
                    best_T = T
        new_ms += round(elapsed * 1000)
        completed |= status == "completed"
        attempts.append(dict(path=str(attempt.relative_to(run)), status=status, elapsed_ms=round(elapsed*1000)))
    if best_T > case["reference_T"]:
        raise ValueError(f"最良解が初期解より悪化した: {case['case']}")
    state.update(T=best_T, new_elapsed_ms=new_ms, elapsed_ms=case["reused_elapsed_ms"] + new_ms,
                 attempts=attempts, best_sha256=search.sha256(base / "best.txt"))
    if completed:
        state["status"] = "reused" if case["reused"] else "completed"
    search.write_json(base / "status.json", state)
    return state


def make_attempt(run, manifest, case):
    base = run / "cases" / case["case"]
    attempts = base / "attempts"
    attempts.mkdir(exist_ok=True)
    index = len(list(attempts.glob("attempt_*")))
    attempt = attempts / f"attempt_{index:03d}"
    # Prepare in a staging directory so an interrupted preparation is never
    # confused with an attempt that actually started searching.
    stage = attempts / f"staging_{uuid.uuid4().hex[:8]}"
    destination = stage / "cases" / case["case"]
    (destination / "seeds").mkdir(parents=True)
    shutil.copy2(run / case["input"], destination / "input.txt")
    best = (base / "best.txt").read_text()
    texts = [best] + [(run / s["path"]).read_text() for s in case["seeds"]]
    seeds, seen = [], set()
    for text in texts:
        if text in seen:
            continue
        seen.add(text)
        path = destination / "seeds" / f"{len(seeds):02d}.txt"
        path.write_text(text)
        seeds.append(dict(path=str(path.relative_to(stage))))
        if len(seeds) == 8:
            break
    config = dict(manifest["config"])
    config["seed"] = int.from_bytes(hashlib.sha256(f"{config['seed']}:{case['case']}:{index}".encode()).digest()[:8], "big")
    item = dict(case=case["case"], input=str((destination / "input.txt").relative_to(stage)),
                reference_T=search.Problem.read(destination / "input.txt").replay(best)["T"], seeds=seeds)
    attempt_manifest = dict(config=config, cases=[item])
    search.write_json(stage / "manifest.json", attempt_manifest)
    stage.rename(attempt)
    return attempt, attempt_manifest


def run_pending(run, manifest, binary):
    search.ROOT = ROOT
    for case in manifest["cases"]:
        state = reconcile_attempts(run, case)
        if state["status"] in ("completed", "reused"):
            continue
        print(f"[{case['case']}] 5分の探索を開始。完了ケースは再探索しない。", flush=True)
        attempt, attempt_manifest = make_attempt(run, manifest, case)
        search.write_json(run / "status.json", dict(status="running", case=case["case"], updated_at=search.utc_now()))
        code = search.run_search(attempt, attempt_manifest, binary)
        state = reconcile_attempts(run, case)
        if code:
            state["status"] = "interrupted" if code == 130 else "failed"
            search.write_json(run / "cases" / case["case"] / "status.json", state)
            search.write_json(run / "status.json", dict(status=state["status"], case=case["case"], updated_at=search.utc_now()))
            return code
    return 0


def csv_add_once(path, header, row, identity):
    """Atomic append with identity checks also permits a retry after partial publication."""
    original = path.read_text() if path.exists() else ""
    if original:
        reader = csv.reader(io.StringIO(original))
        if next(reader) != header:
            raise ValueError(f"CSVの列が通常評価と異なる: {path}")
        found = [old for old in reader if all(old[i] == str(row[i]) for i in identity)]
        if found:
            if found != [[str(v) for v in row]]:
                raise ValueError(f"同じ実行のCSV記録が一致しない: {path}")
            return
    output = io.StringIO(newline="")
    writer = csv.writer(output)
    if not original:
        writer.writerow(header)
    writer.writerow(row)
    search.atomic_text(path, original + ("\n" if original and not original.endswith("\n") else "") + output.getvalue())


def publish(run, manifest, scorer):
    verify_snapshot(run, manifest)
    eval_module = load_module(run / "snapshot/eval.py", "v800_eval_format")
    results, provenance = [], []
    for case in manifest["cases"]:
        base = run / "cases" / case["case"]
        state = reconcile_attempts(run, case)
        if state["status"] not in ("completed", "reused"):
            raise ValueError(f"未完了ケースがあるため登録しない: {case['case']}")
        inp, plan = run / case["input"], base / "best.txt"
        T = search.Problem.read(inp).replay(plan.read_text())["T"]
        if T > case["reference_T"]:
            raise ValueError(f"初期解より悪化: {case['case']}")
        official = subprocess.run([str(scorer), "--no-vis", str(inp), str(plan)],
                                  text=True, capture_output=True, check=True)
        if official.stdout.strip() != f"Score = {T}":
            raise ValueError(f"公式採点と独立再生が一致しない: {case['case']}: {official.stdout}")
        results.append(eval_module.CaseResult(case_name=case["case"] + ".txt", status="ok", score=T,
                                              elapsed=state["elapsed_ms"], stdout_path=f"results/out/v800/{case['case']}.txt"))
        provenance.append(dict(case=case["case"], initial_T=case["reference_T"], T=T,
                               reused=case["reused"], elapsed_ms=state["elapsed_ms"], new_elapsed_ms=state["new_elapsed_ms"],
                               best_source=state["best_source"], best_sha256=search.sha256(plan), official_score=T,
                               frozen_output=str(plan.relative_to(ROOT)) if plan.is_relative_to(ROOT) else str(plan)))
    if len(results) != 100 or [r.case_name for r in results] != [c + ".txt" for c in CASES]:
        raise ValueError("100件揃うまで登録しない")
    records = eval_module.make_records(results, manifest["run_id"], manifest["executed_at"], "v800",
                                       manifest["label"], "tools/in", True)
    for record, source in zip(records, provenance):
        record.update(reference=True, reference_provenance=source)
    records_path = ROOT / "results/eval_records.jsonl"
    original = records_path.read_text() if records_path.exists() else ""
    existing = [json.loads(line) for line in original.splitlines() if line.strip()]
    same = [row for row in existing if row.get("run_id") == manifest["run_id"]]
    if same and same != records:
        raise ValueError("同じrun_idの評価記録が今回の100件と一致しない")
    search.write_json(run / "provenance.json", provenance)
    search.write_json(run / "publication.json", dict(status="publishing", run_id=manifest["run_id"], records=records))
    for case in manifest["cases"]:
        search.atomic_text(ROOT / "results/out/v800" / (case["case"] + ".txt"),
                           (run / "cases" / case["case"] / "best.txt").read_text())
    avg, total, low, high, avg_ms, max_ms = eval_module.summarize(results)
    csv_add_once(ROOT / "results/score_summary.csv", eval_module.SUMMARY_HEADER,
                 ["v800", avg, total, low, high, avg_ms, max_ms, "tools/in", 100, manifest["label"], manifest["executed_at"]],
                 [0, 9, 10])
    detail_header = ["bin", "total_avg", "max_elapsed", *[c + ".txt" for c in CASES], "label", "executed_at"]
    csv_add_once(ROOT / "results/score_detail.csv", detail_header,
                 ["v800", avg, max_ms, *[r.score for r in results], manifest["label"], manifest["executed_at"]], [0, 103, 104])
    # The viewer reads only JSONL. Commit all 100 rows together after outputs
    # and CSVs are durable, so it never sees an incomplete reference run.
    if not same:
        block = "".join(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n" for row in records)
        search.atomic_text(records_path, original + ("\n" if original and not original.endswith("\n") else "") + block)
    search.write_json(run / "publication.json", dict(status="published", run_id=manifest["run_id"], cases=100))
    search.write_json(run / "status.json", dict(status="completed", published=True, updated_at=search.utc_now()))
    print(f"eval viewerへv800を登録した: 100件、平均{total/100:.2f}手、max_elapsed={max_ms} ms", flush=True)


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workers", type=int, choices=[2], default=2, help="同時探索数。2に固定")
    parser.add_argument("--seed", type=int, default=80020260929)
    parser.add_argument("--reuse-run", type=Path, help="再利用する完了済みv047記録。既定は最新の該当記録")
    parser.add_argument("--run-dir", type=Path, help="新規保存先。既存フォルダは上書きしない")
    parser.add_argument("--resume", type=Path, metavar="RUN_DIR", help="完了ケースを省き、未完了ケースを保存最良解から5分やり直す")
    parser.add_argument("--prepare-only", action="store_true", help="保存解を凍結して再生検証する。探索と評価表登録は行わない")
    args = parser.parse_args(argv)
    if not 0 <= args.seed < 2**64:
        parser.error("--seedは64 bit符号なし整数")
    if args.resume and (args.run_dir or args.reuse_run):
        parser.error("--resumeは--run-dir、--reuse-runと併用できない")
    return args


def main(argv=None):
    args = parse_args(argv)
    run_id = datetime.now().strftime("%Y%m%dT%H%M%S") + "_" + uuid.uuid4().hex[:8]
    run = (args.resume or args.run_dir or ROOT / "results/long_search/v800" / run_id).resolve()
    search.ROOT = ROOT
    with search.evaluation_lock():
        # A forcibly killed coordinator cannot reap its detached children.
        # Refuse another search while those children still consume the 2 cores.
        processes = subprocess.check_output(["ps", "-axo", "pid=,command="], text=True)
        if any("/snapshot/v800 --initial-plan " in line for line in processes.splitlines()):
            raise RuntimeError("前回のv800探索プロセスがまだ動いている。終了後に再開する")
        manifest = read_json(run / "manifest.json") if args.resume else prepare(args, run)
        verify_snapshot(run, manifest)
        if args.prepare_only:
            print("準備と保存解の再生確認のみ完了。探索とv800の登録は未実行。")
            return 0
        try:
            binary, scorer = build(run, manifest)
            if run_pending(run, manifest, binary) != 0:
                return 130 if read_json(run / "status.json")["status"] == "interrupted" else 1
            publish(run, manifest, scorer)
            return 0
        except KeyboardInterrupt:
            search.write_json(run / "status.json", dict(status="interrupted", updated_at=search.utc_now()))
            print(f"中断した。再開: python3 adhoc/scripts/run_v800.py --resume {run}", flush=True)
            return 130
        except Exception as exc:
            search.write_json(run / "status.json", dict(status="failed", error=str(exc), updated_at=search.utc_now()))
            raise


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, search.request_interrupt)
    if hasattr(signal, "SIGHUP"):
        signal.signal(signal.SIGHUP, search.request_interrupt)
    try:
        sys.exit(main())
    except (ValueError, OSError, RuntimeError, subprocess.CalledProcessError) as exc:
        print(f"v800: {exc}", file=sys.stderr)
        sys.exit(1)
