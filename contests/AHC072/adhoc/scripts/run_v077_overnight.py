#!/usr/bin/env python3
"""教師生成→検査→固定条件の学習。LLMによる結果依存の変更は行わない。"""
import argparse
from collections import deque
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback

ROOT = Path(__file__).resolve().parents[2]
MILESTONES = [128, 2048, 8192, 32768, 65536]
SOURCES = ["src/bin/v076_relative_tuned.cpp", "adhoc/bin/collect_v077_teacher.cpp",
           "adhoc/scripts/build_v077_teacher.py", "adhoc/scripts/v077_teacher_support.cpp.txt",
           "adhoc/scripts/run_v077_overnight.py", "adhoc/scripts/train_v077_rank.py"]
ACTIVE = {}
LOCK = threading.Lock()

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def atomic_json(path, value):
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n")
    temp.replace(path)

def timestamp():
    return datetime.now(timezone.utc).isoformat()

def stop_processes():
    with LOCK:
        processes = list(ACTIVE.values())
    for process in processes:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    time.sleep(0.2)
    for process in processes:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

def run_process(command, log, *, stdin=None, stdout=None, env=None, timeout=120):
    with log.open("ab") as err:
        process = subprocess.Popen(command, cwd=ROOT, stdin=stdin or subprocess.DEVNULL,
                                   stdout=stdout or err, stderr=err, env=env, start_new_session=True)
        with LOCK:
            ACTIVE[process.pid] = process
        try:
            code = process.wait(timeout=timeout)
            if code:
                raise RuntimeError(f"process exited {code}: {command[0]}; log={log}")
        except BaseException:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
            raise
        finally:
            with LOCK:
                ACTIVE.pop(process.pid, None)

def input_properties(data):
    lines = data.decode().splitlines()
    header = lines[0].split() if lines else []
    if len(header) != 2 or not all(x.isdigit() for x in header):
        return None
    N, K = map(int, header)
    if not (12 <= N <= 20 and 4 <= K <= 12) or len(lines) != N + 1:
        return None
    if any(len(row) != N or any(c not in ".#ABCDEFGHIJKLabcdefghijkl" for c in row) for row in lines[1:]):
        return None
    cells = "".join(lines[1:])
    return {"N": N, "K": K, "M": sum("a" <= c <= "l" for c in cells), "floors": sum(c != "#" for c in cells)}

def prior_inputs(run):
    paths = subprocess.check_output(["rg", "--files", "-uuu", "tools", "results", "adhoc", "samples",
                                    "-g", "*.txt", "-g", "*.json", "-g", "!**/target/**",
                                    "-g", "!**/node_modules/**", "-g", "!**/__pycache__/**"], cwd=ROOT, text=True).splitlines()
    seeds, hashes, references = set(), set(), []
    def visit(value, key=""):
        if isinstance(value, dict):
            for k, v in value.items():
                visit(v, k)
        elif isinstance(value, list):
            for v in value:
                visit(v, key)
        elif isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value):
            hashes.add(value)
        elif "seed" in key.lower() and isinstance(value, (int, str)) and str(value).isdigit():
            seeds.add(int(value))
    for rel in paths:
        path = ROOT / rel
        if run in path.parents:
            continue
        size = path.stat().st_size
        if size > 16 * 1024 * 1024:
            continue
        if path.suffix == ".json" and ("manifest" in path.name or "seed" in path.name or "input" in path.name):
            try:
                visit(json.loads(path.read_text()))
                references.append(rel)
            except (UnicodeError, json.JSONDecodeError):
                continue
        elif path.suffix == ".txt":
            if "seed" in path.name:
                for line in path.read_text().splitlines():
                    if line.strip().isdigit():
                        seeds.add(int(line.strip()))
                references.append(rel)
            elif size < 8192:
                data = path.read_bytes()
                try:
                    props = input_properties(data)
                except UnicodeError:
                    props = None
                if props:
                    hashes.add(hashlib.sha256(data).hexdigest())
    atomic_json(run / "prior_inputs.json", {"seeds": sorted(seeds), "hashes": sorted(hashes), "references": sorted(set(references))})
    return seeds, hashes

class Inputs:
    def __init__(self, run, seeds, hashes):
        self.run, self.excluded_seeds, self.hashes = run, seeds, hashes
        self.items = [json.loads(line) for line in (run / "inputs.jsonl").read_text().splitlines()] if (run / "inputs.jsonl").exists() else []
        generated = [json.loads(line) for line in (run / "generated.jsonl").read_text().splitlines()] if (run / "generated.jsonl").exists() else []
        assigned = {item["sha256"] for item in self.items}
        self.queues = [deque(), deque()]
        self.next_seed = max([910000000 - 1] + [item["seed"] for item in generated]) + 1
        for item in generated:
            self.hashes.add(item["sha256"])
            if item["sha256"] not in assigned:
                self.queues[int(item["M"] >= 80)].append(item)

    def add_batch(self):
        batch_start = self.next_seed
        values = []
        while len(values) < 2048:
            if self.next_seed not in self.excluded_seeds:
                values.append(self.next_seed)
            self.next_seed += 1
        folder = self.run / "generated" / str(batch_start)
        folder.mkdir(parents=True, exist_ok=False)
        seed_file = folder / "seeds.txt"
        seed_file.write_text("".join(f"{seed}\n" for seed in values))
        run_process([str(self.run / "frozen/gen"), str(seed_file), "--dir", str(folder / "in")], self.run / "generation.log", timeout=300)
        with (self.run / "generated.jsonl").open("a") as stream:
            for index, seed in enumerate(values):
                path = folder / "in" / f"{index:04d}.txt"
                data = path.read_bytes()
                sha = hashlib.sha256(data).hexdigest()
                props = input_properties(data)
                if props is None:
                    raise ValueError(f"malformed generated input: {path}")
                if sha in self.hashes:
                    continue
                self.hashes.add(sha)
                item = {"seed": seed, "sha256": sha, "path": str(path.relative_to(self.run)), **props}
                stream.write(json.dumps(item) + "\n")
                self.queues[int(props["M"] >= 80)].append(item)

    def ensure(self, target):
        with (self.run / "inputs.jsonl").open("a") as stream:
            while len(self.items) < target:
                i = len(self.items)
                # 8入力を1ブロックとし、両群それぞれ学習3入力・調整1入力を固定する。
                group = i % 2
                if not self.queues[group]:
                    self.add_batch()
                item = {**self.queues[group].popleft(), "index": i, "role": "train" if i % 8 < 6 else "validation"}
                self.items.append(item)
                stream.write(json.dumps(item) + "\n")
                stream.flush()

def validate_case(folder):
    parent = json.loads((folder / "parent.json").read_text())
    if parent["errors"] or parent["pool_free"] != 4:
        raise ValueError(f"parent mechanism check failed: {folder}")
    states_path, rows_path = folder / "states.jsonl", folder / "candidates.jsonl"
    states = [json.loads(line) for line in states_path.read_text().splitlines()] if states_path.exists() else []
    rows = [json.loads(line) for line in rows_path.read_text().splitlines()] if rows_path.exists() else []
    if not states or parent["snapshots"] != len(states):
        raise ValueError(f"no usable snapshot or incomplete state table: {folder}")
    expected = {(state["phase"], cand["rank"]): (state, cand) for state in states for cand in state["candidates"]}
    seen = set()
    informative = 0
    for row in rows:
        key = (row["phase"], row["rank"])
        if key in seen or key not in expected:
            raise ValueError("duplicate or unexpected candidate")
        seen.add(key)
        state, candidate = expected[key]
        assert row["snapshot_hash"] == state["snapshot_hash"] and row["rng"] == state["rng"]
        assert row["candidate_hash"] == candidate["hash"]
        assert row["E"] == 0 and row["errors"] == 0 and row["pool_free"] == 4
        assert row["best_T"] <= len(state["best"]) and row["best_T"] <= row["current_T"]
        assert row["last_accept_elapsed"] <= row["budget"] + 1e-9
        assert 0 < row["budget"] <= 1.52 * 0.02 + 1e-9
        assert len(row["features"]) == 32 and all(math.isfinite(v) for v in row["features"])
        assert row["features"][5] == len(state["current"]) and row["features"][6] == len(state["best"])
    assert seen == set(expected)
    for state in states:
        group = [row for row in rows if row["phase"] == state["phase"]]
        assert min(row["rank"] for row in group) == state["baseline_rank"]
        informative += len({row["best_T"] for row in group}) > 1
    result = {"snapshots": len(states), "rows": len(rows), "informative_states": informative,
              "missing_phases": 4-len(states), "best_T": parent["best_T"],
              "candidates_sha256": digest(rows_path), "states_sha256": digest(states_path),
              "completed_at": timestamp()}
    return result

def collect(run, item):
    folder = run / "cases" / f"{item['index']:06d}"
    if folder.exists():
        if (folder / "complete.json").exists():
            return json.loads((folder / "complete.json").read_text())
        # 不完全な入力を勝手に再測定せず、明示的な再開判断を求める。
        raise RuntimeError(f"incomplete case already exists: {folder}")
    folder.mkdir(parents=True)
    env = {**os.environ, "NN_TEACHER_DIR": str(folder), "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1"}
    atomic_json(folder / "input.json", item)
    with (run / item["path"]).open("rb") as source, (folder / "parent.out").open("wb") as output:
        run_process([str(run / "frozen/collect_v077_teacher")], folder / "stderr.log", stdin=source, stdout=output, env=env, timeout=180)
    result = validate_case(folder)
    atomic_json(folder / "complete.json", result)
    return result

def pipeline(run, resume):
    run.mkdir(parents=True, exist_ok=True)
    started = time.time()
    hard_end, data_end = started + 5 * 3600, started + 4 * 3600
    config_path = run / "config.json"
    if not resume:
        if config_path.exists():
            raise RuntimeError("run already exists; use --resume after inspecting its status")
        frozen = run / "frozen"
        frozen.mkdir()
        hashes = {}
        for source in SOURCES:
            destination = frozen / source
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / source, destination)
            hashes[source] = digest(ROOT / source)
        for name, original in (("collect_v077_teacher", ROOT / "target/release/collect_v077_teacher"), ("gen", ROOT / "tools/target/release/gen")):
            shutil.copy2(original, frozen / name)
            hashes[name] = digest(original)
        atomic_json(config_path, {"started_at": timestamp(), "started_epoch": started, "hard_end_epoch": hard_end,
                                 "data_end_epoch": data_end, "milestones": MILESTONES, "workers": 30,
                                 "cpu": "Apple M5 Ultra", "local_time_ratio": 0.80,
                                 "source_hashes": hashes, "python": sys.version, "command": sys.argv})
        seeds, hashes = prior_inputs(run)
    else:
        config = json.loads(config_path.read_text())
        for source in SOURCES:
            if digest(ROOT / source) != config["source_hashes"][source]:
                raise RuntimeError(f"source changed: {source}")
        prior = json.loads((run / "prior_inputs.json").read_text())
        seeds, hashes = set(prior["seeds"]), set(prior["hashes"])
        atomic_json(run / f"resume_{int(started)}.json", {"started_epoch": started, "hard_end_epoch": hard_end})
    progress = {"pid": os.getpid(), "run": str(run), "started_at": timestamp(), "hard_end_epoch": hard_end,
                "data_end_epoch": data_end, "stage": "preparing", "completed_inputs": 0, "models": [], "failed": False}
    def update(**values):
        progress.update(values)
        progress["updated_at"] = timestamp()
        atomic_json(run / "status.json", progress)
        print(json.dumps(progress, ensure_ascii=False), flush=True)
    def hard_stop():
        stop_processes()
        atomic_json(run / "exit.json", {"reason": "five_hour_deadline", "exit_code": 124, "at": timestamp()})
        os._exit(124)
    timer = threading.Timer(max(1, hard_end-time.time()), hard_stop)
    timer.daemon = True
    timer.start()
    caffeinate = subprocess.Popen(["/usr/bin/caffeinate", "-i", "-w", str(os.getpid())], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) if sys.platform == "darwin" else None
    completed = {int(path.parent.name) for path in (run / "cases").glob("*/complete.json")}
    update(completed_inputs=len(completed))
    inputs = Inputs(run, seeds, hashes)
    def report():
        lines = ["# v077 夜間実行", "", f"更新: {timestamp()}", "",
                 f"完了入力: {len(completed)}。現在の状態は status.json、終了理由は exit.json を参照。", "",
                 "| 入力数 | モデル | 調整用入力数 | 現行順位との差（手数、負が改善） | 最良epoch |", "|---:|---|---:|---:|---:|"]
        for path in sorted((run / "models").glob("*/summary.json")):
            summary = json.loads(path.read_text())
            for name, row in summary["models"].items():
                lines.append(f"| {summary['inputs']} | {name} | {row['inputs']} | {row['delta_vs_baseline']:.6f} | {row['epoch']} |")
        lines += ["", "上表は短い候補比較の教師値を使った調整用の評価である。提出solverの完成スコアの評価と採否は後続。",
                  "", "入力と用途: inputs.jsonl。局面・候補・合法性の検査: cases/。凍結したコードと実行物: frozen/。",
                  "各段階のデータ量・欠測・期限超過: models/n*/dataset.json。重みと正規化: models/n*/*_weights.json。", ""]
        temp = run / "REPORT.md.tmp"
        temp.write_text("\n".join(lines))
        temp.replace(run / "REPORT.md")
    def collect_to(target):
        inputs.ensure(target)
        pending_items = iter(item for item in inputs.items[:target] if item["index"] not in completed)
        futures = {}
        stopped = False
        with ThreadPoolExecutor(max_workers=30) as pool:
            def submit_next():
                nonlocal stopped
                if time.time() >= data_end:
                    stopped = True
                    return False
                item = next(pending_items, None)
                if item is None:
                    return False
                futures[pool.submit(collect, run, item)] = item
                return True
            for _ in range(30):
                if not submit_next():
                    break
            last_update = 0.0
            while futures:
                finished, _ = wait(futures, timeout=10, return_when=FIRST_COMPLETED)
                try:
                    for future in finished:
                        item = futures.pop(future)
                        result = future.result()
                        completed.add(item["index"])
                        if shutil.disk_usage(run).free < 20 * 1024**3:
                            raise RuntimeError("free disk space is below 20 GiB")
                        submit_next()
                except BaseException:
                    for future in futures:
                        future.cancel()
                    stop_processes()
                    raise
                if time.time() - last_update >= 15 or not futures:
                    update(stage="collecting", target_inputs=target, completed_inputs=len(completed), active_inputs=len(futures))
                    last_update = time.time()
        return not stopped
    def train_count(count):
        summary = run / "models" / f"n{count:06d}" / "summary.json"
        if all((summary.parent / f"{name}_finished.json").exists() for name in ("linear", "mlp16")):
            return
        update(stage="training", training_inputs=count, completed_inputs=len(completed), active_inputs=0)
        command = [sys.executable, str(run / "frozen/adhoc/scripts/train_v077_rank.py"), "--run", str(run),
                   "--count", str(count), "--deadline", str(hard_end - 120)]
        run_process(command, run / "training.log", timeout=max(1, hard_end-time.time()-60))
        progress["models"].append(str(summary.relative_to(run)))
        update(stage="trained")
        report()
    try:
        update(stage="mechanism_check")
        if not collect_to(4):
            raise RuntimeError("data deadline before mechanism check")
        check_phases = set()
        for i in range(4):
            for line in (run / "cases" / f"{i:06d}" / "states.jsonl").read_text().splitlines():
                check_phases.add(json.loads(line)["phase"])
        if check_phases != {0, 1, 2, 3}:
            raise ValueError("four-input mechanism check did not cover all four phases")
        atomic_json(run / "mechanism_check.json", {"passed": True, "indices": [0, 1, 2, 3], "at": timestamp()})
        for target in MILESTONES:
            if time.time() >= data_end:
                break
            reached = collect_to(target)
            # 完了済みの連続した8入力ブロックだけで両群と用途の比率を守る。
            contiguous = 0
            while contiguous in completed:
                contiguous += 1
            usable = contiguous // 8 * 8
            if usable >= 128:
                train_count(min(target, usable))
            if not reached or time.time() >= hard_end-180:
                break
        update(stage="finished", completed_inputs=len(completed), active_inputs=0)
        atomic_json(run / "exit.json", {"reason": "registered_stages_finished", "exit_code": 0, "at": timestamp()})
        report()
    except BaseException as error:
        stop_processes()
        update(stage="failed", failed=True, error=str(error), completed_inputs=len(completed), active_inputs=0)
        atomic_json(run / "exit.json", {"reason": str(error), "traceback": traceback.format_exc(), "exit_code": 1, "at": timestamp()})
        report()
        raise
    finally:
        timer.cancel()
        if caffeinate is not None:
            caffeinate.terminate()

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    pipeline(args.run.resolve(), args.resume)
