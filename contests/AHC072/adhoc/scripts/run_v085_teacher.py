#!/usr/bin/env python3
"""固定した長時間教師の採取を、入力・経路・再出発単位で保存する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback

from run_v077_overnight import input_properties, prior_inputs
from run_v079_experiment import local_log
from v085_data import ROOT, Problem, compress_episodes, now, quality, save, sha, status, validate_episodes

STOP = threading.Event()
ENV = os.environ.copy() | {name: "1" for name in (
    "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS")}
SOURCES = ["src/bin/v079_nn_immediate.cpp", "adhoc/bin/v800.cpp",
           "adhoc/bin/collect_v085_teacher.cpp", "adhoc/bin/v801_teacher.cpp",
           "adhoc/scripts/build_v085_teacher.py", "adhoc/scripts/run_v085_teacher.py",
           "adhoc/scripts/v085_data.py", "adhoc/scripts/run_v047_long_search.py",
           "adhoc/scripts/run_v077_overnight.py", "adhoc/scripts/run_v079_experiment.py",
           "adhoc/scripts/memory_guard.py", "scripts/build_solver.sh",
           "tools/src/bin/vis.rs", "tools/src/lib.rs"]
BINARIES = {"baseline": "v079_nn_immediate", "v800": "collect_v085_teacher", "v801": "v801_teacher"}
OLD = ROOT / "results/nn_rank/v077/20261002T021222_studio"


def command(args, log, deadline, *, input_path=None, output_path=None, timeout=None):
    if STOP.is_set() or time.time() >= deadline:
        raise TimeoutError("registered_time_limit_or_signal")
    deadline = min(deadline, time.time()+timeout) if timeout is not None else deadline
    with log.open("ab") as errors:
        inp = input_path.open("rb") if input_path else subprocess.DEVNULL
        out = output_path.open("xb") if output_path else errors
        try:
            started = time.monotonic()
            # 全子プロセスをmemory_guardの同じ群へ残す。
            process = subprocess.Popen(list(map(str, args)), cwd=ROOT, stdin=inp, stdout=out, stderr=errors, env=ENV)
            try:
                while process.poll() is None:
                    if STOP.is_set() or time.time() >= deadline:
                        raise TimeoutError("registered_time_limit_or_signal")
                    try:
                        process.wait(timeout=.25)
                    except subprocess.TimeoutExpired:
                        pass
                if process.returncode:
                    raise RuntimeError(f"child exit {process.returncode}: {log}")
            except BaseException:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        process.kill(); process.wait()
                raise
            return time.monotonic()-started
        finally:
            if input_path:
                inp.close()
            if output_path:
                out.close()


def score(run, inp, plan, folder, deadline):
    metrics = Problem.read(inp).replay(plan.read_text())
    output = folder / "score.txt"
    command([run / "frozen/vis", inp, plan, "--no-vis"], folder / "score.err", deadline,
            output_path=output, timeout=30)
    assert output.read_text().strip() == f"Score = {metrics['T']}"
    return metrics


def record(folder, values):
    values["artifacts"] = {str(p.relative_to(folder)): sha(p) for p in sorted(folder.rglob("*"))
                           if p.is_file() and p.name != "complete.json"}
    values["completed_at"] = now()
    save(folder / "complete.json", values)
    return values


def completed(folder):
    marker = folder / "complete.json"
    if not marker.exists():
        return None
    row = json.loads(marker.read_text())
    for name, digest in row["artifacts"].items():
        assert sha(folder / name) == digest, f"completed artifact changed: {folder / name}"
    return row


def prepare(run, deadline):
    if (run / "config.json").exists():
        verify(run)
        return
    assert not (run / "frozen").exists(), "partial preparation needs inspection"
    status(run, "building")
    command([sys.executable, ROOT / "adhoc/scripts/build_v085_teacher.py"], run / "build.log", deadline)
    (run / "frozen").mkdir()
    for key, name in BINARIES.items():
        for local in ([True] if key == "baseline" else [True, False]):
            args = [ROOT / "scripts/build_solver.sh"] + ([] if local else ["--no-local"]) + [name]
            command(args, run / "build.log", deadline)
            suffix = "" if local else "_nonlocal"
            shutil.copy2(ROOT / "target/release" / name, run / "frozen" / (key+suffix))
    for name in ("gen", "vis"):
        shutil.copy2(ROOT / "tools/target/release" / name, run / "frozen" / name)
    status(run, "excluding_known_inputs")
    seeds, hashes = prior_inputs(run)
    for name in ("inputs.jsonl", "generated.jsonl"):
        with (OLD / name).open() as stream:
            for line in stream:
                item = json.loads(line)
                seeds.add(item["seed"]); hashes.add(item["sha256"])
    save(run / "exclusions.json", {"seeds": sorted(seeds), "hashes": sorted(hashes)})
    status(run, "generating_512_inputs")
    (run / "inputs").mkdir()
    manifest, seed = [], 960000000
    while len(manifest) < 512:
        batch = []
        while len(batch) < 512-len(manifest):
            if seed not in seeds:
                batch.append(seed)
            seed += 1
        folder = run / "generated" / str(batch[0])
        folder.mkdir(parents=True)
        seed_file = folder / "seeds.txt"
        seed_file.write_text("".join(f"{n}\n" for n in batch))
        command([run / "frozen/gen", seed_file, "--dir", folder / "inputs"], run / "generation.log", deadline)
        for i, value in enumerate(batch):
            source = folder / "inputs" / f"{i:04d}.txt"
            digest = sha(source)
            seeds.add(value)
            if digest in hashes:
                continue
            hashes.add(digest)
            props = input_properties(source.read_bytes())
            assert props is not None
            index = len(manifest)
            dest = run / "inputs" / f"{index:06d}.txt"
            shutil.copyfile(source, dest)
            manifest.append({"index": index, "seed": value, "sha256": digest,
                             "role": "validation" if index % 4 == 0 else "train",
                             "path": str(dest.relative_to(run)), **props})
    assert len({m["sha256"] for m in manifest}) == 512
    save(run / "input_manifest.json", manifest)
    probes = json.loads((ROOT / "results/nn_rank/v079/20261002T104039_studio/probe_manifest.json").read_text())[:2]
    assert probes[0]["M"] < 80 <= probes[1]["M"]
    for item in probes:
        dest = run / "probes" / f"{item['index']:06d}.txt"
        dest.parent.mkdir(exist_ok=True)
        shutil.copyfile(ROOT / item["source"], dest)
        assert sha(dest) == item["sha256"]
        item["path"] = str(dest.relative_to(run))
    save(run / "probe_manifest.json", probes)
    supports = [str(p.relative_to(ROOT)) for p in (ROOT / "adhoc/scripts").glob("v085*.cpp.txt")]
    supports += [str(p.relative_to(ROOT)) for p in (ROOT / "adhoc/scripts").glob("v801*.cpp.txt")]
    sources = sorted(set(SOURCES+supports))
    for rel in sources + ["notes/experiments/v085.md", "notes/experiments/v801.md"]:
        dest = run / "frozen" / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / rel, dest)
    config = {"prepared_at": now(), "source_sha256": {p: sha(ROOT / p) for p in sources},
              "frozen_sha256": {str(p.relative_to(run)): sha(p) for p in (run / "frozen").rglob("*") if p.is_file()},
              "manifest_sha256": sha(run / "input_manifest.json"),
              "probe_manifest_sha256": sha(run / "probe_manifest.json"),
              "teacher_workers": 30, "baseline_workers": 20, "pilot_inputs": 128, "maximum_inputs": 512,
              "v800_seconds": 300, "v801_rounds": 5, "v801_round_seconds": 60,
              "mechanism_seconds": 5, "maximum_seconds": 14400, "memory_stop_bytes": 56000000000,
              "mean_saved_gate": 10, "ci95_lower_gate": 5,
              "platform": subprocess.check_output(["uname", "-a"], text=True).strip(),
              "chip": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
              "compiler": subprocess.check_output([os.environ.get("CXX", "g++-15"), "--version"], text=True)}
    save(run / "config.json", config)
    status(run, "prepared")


def verify(run):
    config = json.loads((run / "config.json").read_text())
    for rel, digest in config["source_sha256"].items():
        assert sha(ROOT / rel) == digest, f"source changed: {rel}"
    for rel, digest in config["frozen_sha256"].items():
        assert sha(run / rel) == digest, f"frozen artifact changed: {rel}"
    assert sha(run / "input_manifest.json") == config["manifest_sha256"]
    assert sha(run / "probe_manifest.json") == config["probe_manifest_sha256"]
    return config


def baseline(run, item, deadline, mechanism=False):
    case = run / ("mechanism" if mechanism else "cases") / f"{item['index']:06d}"
    folder = case / "baseline"
    previous = completed(folder)
    if previous:
        assert previous["input_sha256"] == item["sha256"]
        return previous
    folder.mkdir(parents=True, exist_ok=False)
    inp = run / item["path"]
    assert sha(inp) == item["sha256"]
    seconds = command([run / "frozen/baseline"], folder / "stderr.log", deadline,
                      input_path=inp, output_path=folder / "output.txt", timeout=60)
    metrics = score(run, inp, folder / "output.txt", folder, deadline)
    trace = local_log(folder / "stderr.log", metrics["T"])
    return record(folder, {**metrics, "input_sha256": item["sha256"], "wall_seconds": seconds,
                          "binary_sha256": sha(run / "frozen/baseline"), "trace": trace})


def seed_for(index, mode, round_id):
    text = f"v085:85001:{index}:{mode}:{round_id}"
    return int.from_bytes(hashlib.sha256(text.encode()).digest()[:8], "big")


def round_once(run, item, mode, round_id, initial, seconds, deadline, *, mechanism=False, nonlocal_build=False):
    case = run / ("mechanism" if mechanism else "cases") / f"{item['index']:06d}"
    name = mode + ("_nonlocal" if nonlocal_build else "")
    folder = case / name / f"round_{round_id:02d}"
    binary = run / "frozen" / name
    initial_hash = sha(initial)
    previous = completed(folder)
    if previous:
        assert previous["input_sha256"] == item["sha256"] and previous["initial_sha256"] == initial_hash
        assert previous["binary_sha256"] == sha(binary) and previous["requested_seconds"] == seconds
        return previous
    if shutil.disk_usage(run).free < 20*1024**3:
        raise RuntimeError("disk_space_below_20_GiB")
    folder.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(initial, folder / "initial.txt")
    inp = run / item["path"]
    assert sha(inp) == item["sha256"]
    seed = seed_for(item["index"], name, round_id)
    save(folder / "started.json", {"started_at": now(), "seed": str(seed), "seconds": seconds,
         "initial_sha256": initial_hash, "input_sha256": item["sha256"], "binary_sha256": sha(binary)})
    elapsed = command([binary, "--initial-plan", folder / "initial.txt", "--output-dir", folder / "search",
                       "--seconds", seconds, "--seed", seed], folder / "stderr.log", deadline,
                      input_path=inp, output_path=folder / "output.txt", timeout=seconds+60)
    metrics = score(run, inp, folder / "output.txt", folder, deadline)
    initial_T = len(initial.read_text().splitlines())
    assert metrics["T"] <= initial_T
    assert sha(folder / "output.txt") == sha(folder / "search/best.txt")
    events = [json.loads(line) for line in (folder / "search/events.jsonl").read_text().splitlines()]
    assert events[0]["type"] == "start" and events[-1]["type"] == "finish"
    assert events[-1]["status"] in ("completed", "time_limit")
    assert events[-1]["T"] == metrics["T"] and events[-1]["E"] == 0
    assert events[-1]["elapsed_sec"] >= seconds*.98, "teacher did not use the long search budget"
    improvements = [r for r in events if r["type"] in ("initial", "improvement")]
    assert improvements and improvements[0]["T"] == initial_T
    assert all(b["T"] < a["T"] for a, b in zip(improvements, improvements[1:]))
    audit = validate_episodes(folder / "search", inp)
    stats = json.loads((folder / "search/teacher_stats.json").read_text())
    compress_episodes(folder / "search")
    return record(folder, {**metrics, "initial_T": initial_T, "initial_sha256": initial_hash,
                          "input_sha256": item["sha256"], "binary_sha256": sha(binary),
                          "requested_seconds": seconds, "wall_seconds": elapsed, "seed": str(seed),
                          "episode_audit": audit, "teacher_stats": stats})


def mechanism(run, deadline):
    if (run / "mechanism.json").exists():
        assert json.loads((run / "mechanism.json").read_text())["passed"]
        return
    status(run, "mechanism_checks")
    probes = json.loads((run / "probe_manifest.json").read_text())
    rows = []
    for item in probes:
        baseline(run, item, deadline, True)
        initial = run / "mechanism" / f"{item['index']:06d}" / "baseline/output.txt"
        for mode in ("v800", "v801"):
            for nonlocal_build in (False, True):
                result = round_once(run, item, mode, 0, initial, 5, deadline,
                                    mechanism=True, nonlocal_build=nonlocal_build)
                rows.append({"index": item["index"], "M": item["M"], "mode": mode,
                             "nonlocal": nonlocal_build, **result})
    for mode in ("v800", "v801"):
        selected = [r for r in rows if r["mode"] == mode]
        assert sum(r["teacher_stats"]["changed"] for r in selected) > 0
        assert sum(r["episode_audit"]["episodes"] for r in selected) > 0
        assert sum(r["episode_audit"]["neutral"] for r in selected) > 0
    modern = [r for r in rows if r["mode"] == "v801" and not r["nonlocal"]]
    assert sum(r["teacher_stats"]["nn_calls"] for r in modern) > 0
    assert sum(r["teacher_stats"]["search_reduction_candidates"] for r in modern) > 0
    save(run / "mechanism.json", {"passed": True, "completed_at": now(), "runs": rows})
    status(run, "mechanism_passed")


def pool(run, stage, tasks, fn, workers, deadline):
    status(run, stage, total=len(tasks), completed=0, workers=workers)
    count, started = 0, time.monotonic()
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = [executor.submit(fn, task) for task in tasks]
        try:
            for future in as_completed(futures):
                future.result()
                count += 1
                elapsed = time.monotonic()-started
                status(run, stage, total=len(tasks), completed=count, workers=workers,
                       elapsed_seconds=elapsed, estimated_remaining_seconds=elapsed*(len(tasks)-count)/count)
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
        except BaseException:
            STOP.set()
            for future in futures:
                future.cancel()
            raise


def branch(run, item, mode, deadline):
    initial = run / "cases" / f"{item['index']:06d}" / "baseline/output.txt"
    for round_id in range(1 if mode == "v800" else 5):
        result = round_once(run, item, mode, round_id, initial, 300 if mode == "v800" else 60, deadline)
        initial = run / "cases" / f"{item['index']:06d}" / mode / f"round_{round_id:02d}" / "output.txt"
        assert result["T"] == len(initial.read_text().splitlines())


def finish_cases(run, items):
    for item in items:
        folder = run / "cases" / f"{item['index']:06d}"
        if (folder / "complete.json").exists():
            continue
        paths = [folder / "baseline/output.txt", folder / "v800/round_00/output.txt", folder / "v801/round_04/output.txt"]
        lengths = [len(p.read_text().splitlines()) for p in paths]
        selected = min(range(3), key=lambda k: lengths[k])
        shutil.copyfile(paths[selected], folder / "best.txt")
        # 完了済みの各roundの検査結果へ参照を残し、巨大なファイル一覧は重複させない。
        reports = [folder / "baseline/complete.json", folder / "v800/round_00/complete.json"]
        reports += [folder / f"v801/round_{i:02d}/complete.json" for i in range(5)]
        assert all(p.exists() for p in reports)
        audits = [json.loads(p.read_text()) for p in reports[1:]]
        save(folder / "complete.json", {"input_sha256": item["sha256"], "baseline_T": lengths[0],
             "v800_T": lengths[1], "v801_T": lengths[2], "T": lengths[selected],
             "source": ("baseline", "v800", "v801")[selected], "best_sha256": sha(folder / "best.txt"),
             "reports": {str(p.relative_to(folder)): sha(p) for p in reports},
             "unique_transitions": sum(r["episode_audit"]["unique_transitions"] for r in audits),
             "episodes": sum(r["episode_audit"]["episodes"] for r in audits), "completed_at": now()})


def main_run(run, deadline):
    assert json.loads((run / "mechanism.json").read_text())["passed"]
    manifest = json.loads((run / "input_manifest.json").read_text())
    for begin, end in ((0, 128), (128, 512)):
        if begin and not json.loads((run / "quality_128.json").read_text())["gate"]["passed"]:
            status(run, "completed_quality_gate_failed", inputs=128)
            return
        items = manifest[begin:end]
        warmup = run / f"warmup_{end}"
        if not warmup.exists():
            warmup.mkdir()
            command([run / "frozen/baseline"], warmup / "stderr.log", deadline,
                    input_path=run / items[0]["path"], output_path=warmup / "output.txt", timeout=60)
            score(run, run / items[0]["path"], warmup / "output.txt", warmup, deadline)
        pool(run, f"baselines_{end}", items, lambda item: baseline(run, item, deadline), 20, deadline)
        tasks = [(item, mode) for item in items for mode in ("v800", "v801")]
        pool(run, f"teachers_{end}", tasks, lambda task: branch(run, *task, deadline), 30, deadline)
        status(run, f"validating_{end}")
        finish_cases(run, items)
        report = quality(run, end)
        status(run, f"quality_{end}", mean_saved=report["mean_saved"], ci95=report["ci95"], gate=report["gate"])
    status(run, "completed", inputs=512, quality_passed=report["gate"]["passed"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--mode", choices=("prepare", "check", "main"), required=True)
    parser.add_argument("--seconds", type=int, default=14340)
    args = parser.parse_args()
    assert 0 < args.seconds <= 14400
    run = args.run.resolve()
    run.mkdir(parents=True, exist_ok=True)
    deadline = time.time()+args.seconds
    for sig in (signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, lambda *_: STOP.set())
    with (ROOT / "results/.eval.lock").open("a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        lock.seek(0); lock.truncate()
        lock.write(json.dumps({"tool": "v085_teacher", "pid": os.getpid(), "run": str(run), "started_at": now()})); lock.flush()
        if args.mode == "prepare":
            prepare(run, deadline)
        else:
            verify(run)
            if args.mode == "check":
                mechanism(run, deadline)
            else:
                old_status = json.loads((run / "status.json").read_text())
                if old_status["stage"] in ("completed", "completed_quality_gate_failed"):
                    print("run is already complete; no computation started", flush=True)
                    return
                main_run(run, deadline)


if __name__ == "__main__":
    try:
        main()
    except BaseException as error:
        STOP.set()
        if "--run" in sys.argv:
            run = Path(sys.argv[sys.argv.index("--run")+1]).resolve()
            if run.is_dir():
                save(run / "failure.json", {"error": repr(error), "at": now(), "traceback": traceback.format_exc()})
                status(run, "incomplete", error=repr(error))
        raise
