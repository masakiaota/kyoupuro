#!/usr/bin/env python3
"""事前登録済みの採取→品質判定→2教師の学習を再開可能な単位で進める。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
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
from run_v079_experiment import replay, local_log
from v083_data import ROOT, assemble, evaluate_models, now, quality, save, sha, status, validate_case

STOP = threading.Event()
SOURCES = ["src/bin/v079_nn_immediate.cpp", "adhoc/bin/collect_v083_teacher.cpp",
           "adhoc/scripts/build_v083_teacher.py", "adhoc/scripts/v083_teacher_support.cpp.txt",
           "adhoc/scripts/run_v083_teacher.py", "adhoc/scripts/v083_data.py", "adhoc/scripts/train_v083_teacher.py",
           "adhoc/scripts/memory_guard.py", "adhoc/scripts/v080_data.py", "adhoc/scripts/train_v080_scaling.py",
           "adhoc/scripts/train_v078_targets.py", "adhoc/scripts/train_v077_rank.py",
           "adhoc/scripts/run_v077_overnight.py", "adhoc/scripts/run_v079_experiment.py", "scripts/build_solver.sh"]
OLD = ROOT / "results/nn_rank/v077/20261002T021222_studio"
PROBES = ROOT / "results/nn_rank/v079/20261002T104039_studio/probe_manifest.json"


def terminate_tree(process):
    """同じ監視対象群の中で、この採取器とそのfork子だけを終了する。"""
    try:
        os.kill(process.pid, signal.SIGSTOP)
    except ProcessLookupError:
        process.wait()
        return
    listing = subprocess.check_output(["ps", "-axo", "pid=,ppid="], text=True)
    pairs = [tuple(map(int, line.split())) for line in listing.splitlines()]
    ids = {process.pid}
    while True:
        expanded = ids | {pid for pid, ppid in pairs if ppid in ids}
        if expanded == ids:
            break
        ids = expanded
    for pid in ids:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    process.wait()


def command(args, log, deadline, *, inp=None, out=None, env=None, timeout=None):
    deadline = min(deadline, time.time()+timeout) if timeout is not None else deadline
    with log.open("ab") as errors:
        process = subprocess.Popen(list(map(str, args)), cwd=ROOT, stdin=inp or subprocess.DEVNULL,
                                   stdout=out or errors, stderr=errors, env=env)
        try:
            while process.poll() is None:
                if STOP.is_set() or time.time() >= deadline:
                    raise TimeoutError("registered_time_limit_or_signal")
                try:
                    process.wait(timeout=.25)
                except subprocess.TimeoutExpired:
                    pass
            if process.returncode == 75:
                raise TimeoutError("registered_time_limit")
            if process.returncode:
                raise RuntimeError(f"child exit {process.returncode}: {log}")
        except BaseException:
            terminate_tree(process)
            raise


def prepare(run, deadline):
    if (run / "config.json").exists():
        verify(run)
        return
    if list(run.glob("cases/*")):
        raise RuntimeError("unconfigured run has existing cases")
    status(run, "preparing")
    # 既に実測済みのプロセス群監視をそのまま使用する。
    baseline = ROOT / "results/nn_rank/v080/20261002T113935_studio"
    old_config = json.loads((baseline / "config.json").read_text())
    assert sha(ROOT / "adhoc/scripts/memory_guard.py") == old_config["source_sha256"]["adhoc/scripts/memory_guard.py"]
    assert json.loads((baseline / "memory_checks/passed.json").read_text())["passed"]
    command([sys.executable, ROOT / "adhoc/scripts/build_v083_teacher.py"], run / "build.log", deadline)
    command([ROOT / "scripts/build_solver.sh", "collect_v083_teacher"], run / "build.log", deadline)
    status(run, "excluding_known_inputs")
    seeds, hashes = prior_inputs(run)
    for name in ("inputs.jsonl", "generated.jsonl"):
        with (OLD / name).open() as stream:
            for line in stream:
                item = json.loads(line)
                seeds.add(item["seed"])
                hashes.add(item["sha256"])
    # 別のv083実行で生成済みの入力も明示的に除外する。
    for path in (ROOT / "results/nn_rank/v083").glob("*/input_manifest.json"):
        if path.parent != run:
            for item in json.loads(path.read_text()):
                seeds.add(item["seed"])
                hashes.add(item["sha256"])
    save(run / "exclusions.json", {"seeds": sorted(seeds), "hashes": sorted(hashes)})
    status(run, "generating_2048_inputs")
    (run / "inputs").mkdir(exist_ok=True)
    manifest, seed = [], 940000000
    while len(manifest) < 2048:
        batch = []
        while len(batch) < 2048-len(manifest):
            if seed not in seeds:
                batch.append(seed)
            seed += 1
        folder = run / "generated" / str(batch[0])
        folder.mkdir(parents=True, exist_ok=False)
        seed_file = folder / "seeds.txt"
        seed_file.write_text("".join(f"{n}\n" for n in batch))
        command([ROOT / "tools/target/release/gen", seed_file, "--dir", folder / "in"], run / "generation.log", deadline)
        for i, value in enumerate(batch):
            path = folder / "in" / f"{i:04d}.txt"
            digest = sha(path)
            seeds.add(value)
            if digest in hashes:
                continue
            hashes.add(digest)
            props = input_properties(path.read_bytes())
            assert props is not None
            index = len(manifest)
            dest = run / "inputs" / f"{index:06d}.txt"
            shutil.copyfile(path, dest)
            manifest.append({"index": index, "role": "validation" if index % 4 == 3 else "train",
                             "seed": value, "sha256": digest, "path": str(dest.relative_to(run)), **props})
    assert len(manifest) == len({m["sha256"] for m in manifest}) == 2048
    assert sum(m["role"] == "validation" for m in manifest) == 512
    save(run / "input_manifest.json", manifest)
    probes = json.loads(PROBES.read_text())[:2]
    assert [p["index"] for p in probes] == [8, 9] and probes[0]["M"] < 80 <= probes[1]["M"]
    for item in probes:
        destination = run / "probes" / f"{item['index']:06d}.txt"
        destination.parent.mkdir(exist_ok=True)
        shutil.copyfile(ROOT / item["source"], destination)
        assert sha(destination) == item["sha256"]
        item["path"] = str(destination.relative_to(run))
    save(run / "probe_manifest.json", probes)
    for rel in SOURCES + ["notes/experiments/v083.md", "tools/src/bin/vis.rs"]:
        destination = run / "frozen" / rel
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / rel, destination)
    for name, path in (("collector", "target/release/collect_v083_teacher"), ("gen", "tools/target/release/gen"), ("vis", "tools/target/release/vis")):
        shutil.copy2(ROOT / path, run / "frozen" / name)
    config = {"prepared_at": now(), "source_sha256": {p: sha(ROOT / p) for p in SOURCES},
              "frozen_sha256": {str(p.relative_to(run)): sha(p) for p in (run / "frozen").rglob("*") if p.is_file()},
              "manifest_sha256": sha(run / "input_manifest.json"), "probe_manifest_sha256": sha(run / "probe_manifest.json"),
              "replicas": 8, "steps": 512, "phases": 4, "candidates": 32, "jobs": 20,
              "train_inputs": 1536, "validation_inputs": 512, "pilot_inputs": 128,
              "training_steps": 11520, "seed": 78001, "bootstrap_seed": 83002, "bootstrap_replicates": 6000,
              "maximum_seconds": 28800, "memory_stop_bytes": 56000000000, "gpu_cap_bytes": 16000000000,
              "platform": subprocess.check_output(["uname", "-a"], text=True).strip(),
              "chip": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()}
    save(run / "config.json", config)
    save(run / "gpu_state.json", {"active": False, "updated_unix": time.time(), "driver_bytes": 0})
    status(run, "prepared")


def verify(run):
    config = json.loads((run / "config.json").read_text())
    for rel, digest in config["source_sha256"].items():
        assert sha(ROOT / rel) == digest, f"source changed: {rel}"
    for rel, digest in config["frozen_sha256"].items():
        assert sha(run / rel) == digest, f"frozen file changed: {rel}"
    assert sha(run / "input_manifest.json") == config["manifest_sha256"]
    assert sha(run / "probe_manifest.json") == config["probe_manifest_sha256"]
    return config


def collect_one(run, item, deadline, mechanism=False):
    folder = run / ("mechanism" if mechanism else "cases") / f"{item['index']:06d}"
    marker = folder / "complete.json"
    if marker.exists():
        result = json.loads(marker.read_text())
        assert result["input_sha256"] == item["sha256"] and result["collector_sha256"] == sha(run / "frozen/collector")
        assert sha(run / item["path"]) == item["sha256"]
        for name, digest in result["sha256"].items():
            assert sha(folder / name) == digest
        return result
    # 未完了の入力は勝手に作り直さない。既存の部分記録と原因を確認してから再開する。
    folder.mkdir(parents=True, exist_ok=False)
    save(folder / "input.json", item)
    assert sha(run / item["path"]) == item["sha256"]
    env = os.environ.copy()
    env.update(NN083_DIR=str(folder), NN083_INDEX=str(item["index"]), NN083_REPLICAS="2" if mechanism else "8",
               NN083_STEPS="512", NN083_PHASES="1" if mechanism else "4",
               NN083_CANDIDATES="2" if mechanism else "32", NN083_REPEAT_FIRST="1" if mechanism else "0")
    started = time.monotonic()
    with (run / item["path"]).open("rb") as inp, (folder / "output.txt").open("xb") as out:
        command([run / "frozen/collector"], folder / "stderr.log", deadline, inp=inp, out=out, env=env)
    result, _, _ = validate_case(folder, item, replicas=2 if mechanism else 8, phases=1 if mechanism else 4,
                                 candidates=2 if mechanism else 32, repeat=mechanism)
    T = replay(run / item["path"], folder / "output.txt")
    trace = local_log(folder / "stderr.log", T)
    with (folder / "score.txt").open("xb") as out:
        command([run / "frozen/vis", run / item["path"], folder / "output.txt", "--no-vis"],
                folder / "score.err", deadline, out=out, timeout=30)
    assert (folder / "score.txt").read_text().strip() == f"Score = {T}"
    assert T == result["best_T"]
    result.update(input_sha256=item["sha256"], collector_sha256=sha(run / "frozen/collector"),
                  wall_seconds=time.monotonic()-started, parent_legal=True, completed_at=now(),
                  parent_nn_calls=trace["counts"].get("nn_calls", 0))
    result["sha256"].update({name: sha(folder / name) for name in ("input.json", "output.txt", "stderr.log", "score.txt")})
    save(marker, result)
    return result


def calibrate(run, deadline):
    if (run / "calibration.json").exists():
        assert json.loads((run / "calibration.json").read_text())["passed"]
        return
    status(run, "mechanism_2_inputs")
    results = [collect_one(run, item, deadline, True) for item in json.loads((run / "probe_manifest.json").read_text())]
    rate = sum(r["branch_wall_seconds"] for r in results) / sum(r["branches"] for r in results)
    # 低M/高M各1件からの粗い見積もり。本採取の完了入力から随時更新する。
    seconds_per_input = 4*32*8*(rate+.002)+2
    save(run / "calibration.json", {"passed": True, "cases": results, "branch_seconds": rate,
          "pilot_estimated_seconds": 128*seconds_per_input/20,
          "full_estimated_seconds": 2048*seconds_per_input/20+240,
          "estimate_limitation": "two old training inputs only; later phases and machine contention may differ"})
    status(run, "calibrated")


def collect(run, items, deadline, stage):
    status(run, stage, completed=0, total=len(items), jobs=20)
    started, total_wall, done = time.monotonic(), 0., 0
    iterator, pending = iter(items), {}
    with ThreadPoolExecutor(max_workers=20) as pool:
        try:
            for _ in range(min(20, len(items))):
                item = next(iterator)
                pending[pool.submit(collect_one, run, item, deadline)] = item
            while pending:
                finished, _ = wait(pending, timeout=1, return_when=FIRST_COMPLETED)
                if STOP.is_set() or time.time() >= deadline:
                    raise TimeoutError("registered_time_limit_or_signal")
                for future in finished:
                    pending.pop(future)
                    result = future.result()
                    total_wall += result["wall_seconds"]
                    done += 1
                    rate = total_wall/done
                    status(run, stage, completed=done, total=len(items), jobs=20,
                           mean_input_seconds=rate, remaining_seconds=(len(items)-done)*rate/20,
                           stage_wall_seconds=time.monotonic()-started)
                    item = next(iterator, None)
                    if item is not None:
                        pending[pool.submit(collect_one, run, item, deadline)] = item
        except BaseException:
            STOP.set()
            for future in pending:
                future.cancel()
            raise


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--mode", choices=("prepare", "calibrate", "run"), required=True)
    parser.add_argument("--seconds", type=int, default=28800)
    args = parser.parse_args()
    run = args.run.resolve()
    run.mkdir(parents=True, exist_ok=True)
    deadline = time.time()+args.seconds-60
    signal.signal(signal.SIGTERM, lambda *_: STOP.set())
    signal.signal(signal.SIGINT, lambda *_: STOP.set())
    try:
        if args.mode == "prepare":
            prepare(run, deadline)
            return
        verify(run)
        calibrate(run, deadline)
        if args.mode == "calibrate":
            return
        if (run / "finished.json").exists():
            raise RuntimeError("run already finished; do not repeat it")
        items = json.loads((run / "input_manifest.json").read_text())
        collect(run, items[:128], deadline, "collecting_pilot")
        status(run, "teacher_quality_analysis")
        pilot = assemble(run, items[:128], run / "pilot_data", deadline)
        report = quality(pilot)
        save(run / "quality.json", report)
        del pilot
        if not report["gate"]:
            save(run / "finished.json", {"finished_at": now(), "stage": "quality_gate_failed", "inputs": 128, "training_run": False})
            status(run, "completed_quality_gate_failed")
            return
        collect(run, items[128:], deadline, "collecting_full")
        status(run, "assembling_training_data")
        data = assemble(run, items, run / "data", deadline)
        del data
        status(run, "training_two_teachers")
        command([sys.executable, ROOT / "adhoc/scripts/train_v083_teacher.py", "--run", run,
                 "--deadline", str(deadline-10)], run / "training.log", deadline)
        save(run / "gpu_state.json", {"active": False, "updated_unix": time.time(), "driver_bytes": 0})
        status(run, "evaluating_saved_candidates")
        report = evaluate_models(run)
        save(run / "evaluation.json", report)
        save(run / "finished.json", {"finished_at": now(), "stage": "completed", "inputs": 2048,
                                      "training_run": True, "learning_gate": report["gate"]})
        status(run, "completed", learning_gate=report["gate"])
    except BaseException as error:
        save(run / "failure.json", {"failed_at": now(), "mode": args.mode, "error": repr(error), "traceback": traceback.format_exc()})
        status(run, "stopped", error=repr(error))
        raise


if __name__ == "__main__":
    main()
