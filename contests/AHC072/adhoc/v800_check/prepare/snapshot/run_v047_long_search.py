#!/usr/bin/env python3
"""v047の長時間探索を、1ケースずつ・最大2探索プロセスで実行する。"""

from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import csv
from dataclasses import dataclass
from datetime import datetime, timezone
import fcntl
import hashlib
import io
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "adhoc/bin/v047_long_search.cpp"
DEFAULT_CASES = ["0035", "0033", "0023", "0064", "0060", "0066", "0068", "0005", "0097", "0019"]
REASONS = {
    "0035": "通常99件で最少の13匹", "0033": "壁なし・9色", "0023": "12色・26匹・色の偏りが小さい",
    "0064": "4色・最多色が約70%", "0060": "疎な配置・多い壁・既存の連携比較対象",
    "0066": "通常99件で最大の壁率", "0068": "大盤面・4色・157匹", "0005": "高密度・多い壁",
    "0097": "保存版間に大きな手数差", "0019": "大盤面・12色・217匹",
}
MODES = ("continuous", "multistart")
DIRECTIONS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
THREAD_ENV = {name: "1" for name in (
    "OMP_NUM_THREADS", "OMP_THREAD_LIMIT", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS",
)}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def atomic_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_name(f"{path.name}.{os.getpid()}.{threading.get_ident()}.tmp")
    with temp.open("w", encoding="utf-8", newline="") as handle:
        handle.write(text)
        handle.flush()
    os.replace(temp, path)


def write_json(path: Path, value) -> None:
    atomic_text(path, json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


@dataclass
class Problem:
    N: int
    K: int
    C: list[str]
    initial: dict[tuple[int, int], int]
    nests: dict[tuple[int, int], int]

    @classmethod
    def read(cls, path: Path) -> "Problem":
        tokens = path.read_text().split()
        if len(tokens) < 2:
            raise ValueError(f"入力が空: {path}")
        N, K = map(int, tokens[:2])
        C = tokens[2:]
        if not (12 <= N <= 20 and 4 <= K <= 12 and len(C) == N and all(len(r) == N for r in C)):
            raise ValueError(f"盤面寸法が不正: {path}")
        allowed = set("#." + "abcdefghijkl"[:K] + "ABCDEFGHIJKL"[:K])
        if any(c not in allowed for row in C for c in row):
            raise ValueError(f"盤面の文字が不正: {path}")
        initial = {(i, j): ord(c) - ord("a") for i, row in enumerate(C) for j, c in enumerate(row) if "a" <= c <= "l"}
        nests = {(i, j): ord(c) - ord("A") for i, row in enumerate(C) for j, c in enumerate(row) if "A" <= c <= "L"}
        if Counter(nests.values()) != Counter(range(K)) or set(initial.values()) != set(range(K)):
            raise ValueError(f"巣または色数が不正: {path}")
        floor = {(i, j) for i in range(N) for j in range(N) if C[i][j] != "#"}
        if len(floor) < 2 * K:
            raise ValueError(f"床が少なすぎる: {path}")
        reached = {next(iter(floor))}
        todo = list(reached)
        for i, j in todo:
            for di, dj in DIRECTIONS.values():
                q = (i + di, j + dj)
                if q in floor and q not in reached:
                    reached.add(q)
                    todo.append(q)
        if reached != floor:
            raise ValueError(f"床が連結でない: {path}")
        return cls(N, K, C, initial, nests)

    def features(self) -> dict:
        walls = sum(row.count("#") for row in self.C)
        M = len(self.initial)
        return dict(N=self.N, K=self.K, M=M, wall_fraction=walls / (self.N * self.N),
                    density=M / (self.N * self.N - walls - self.K),
                    dominant_fraction=max(Counter(self.initial.values()).values()) / M)

    def replay(self, text: str) -> dict:
        """操作を生成せず、保存された列を独立に検証する。"""
        towers = {p: [c] for p, c in self.initial.items()}
        T = W = mixed = long_jumps = 0
        for line in text.splitlines():
            if not line.strip():
                continue
            parts = line.split()
            if len(parts) != 5:
                raise ValueError(f"操作{T}: 列数が不正")
            i, j, k, d, length = parts
            i, j, k, length = map(int, (i, j, k, length))
            p = (i, j)
            tower = towers.get(p, [])
            if d not in DIRECTIONS or not (0 <= k < len(tower) and 1 <= length <= k + 1):
                raise ValueError(f"操作{T}: 出発・分割・飛距離が不正")
            di, dj = DIRECTIONS[d]
            for step in range(length + 1):
                r, c = i + step * di, j + step * dj
                if not (0 <= r < self.N and 0 <= c < self.N) or self.C[r][c] == "#":
                    raise ValueError(f"操作{T}: 壁または盤外")
            q = (i + length * di, j + length * dj)
            flight = tower[k:]
            if len(towers.get(q, [])) + len(flight) > 8:
                raise ValueError(f"操作{T}: 帰巣前の容量超過")
            W += len(flight) * length
            mixed += len(set(flight)) > 1
            long_jumps += length > 1
            towers[p] = tower[:k]
            towers.setdefault(q, []).extend(reversed(flight))
            for v in (p, q):
                while towers[v] and towers[v][-1] == self.nests.get(v):
                    towers[v].pop()
            T += 1
            if T > 100000:
                raise ValueError("操作数が100000を超える")
        E = sum(map(len, towers.values()))
        if E:
            raise ValueError(f"{E}匹が未帰巣")
        return dict(T=T, E=E, W=W, mixed_moves=mixed, long_jumps=long_jumps)


def canonical_plan(text: str) -> str:
    return "".join(" ".join(row.split()) + "\n" for row in text.splitlines() if row.strip())


def prepare(args, run: Path) -> dict:
    run.mkdir(parents=True, exist_ok=False)
    (run / "snapshot").mkdir()
    manifest = dict(schema_version=1, experiment="v047", created_at=utc_now(), status="preparing",
                    host=dict(platform=platform.platform(), python=sys.version),
                    config=dict(minutes_per_case=args.minutes_per_case, workers=2,
                                restart_seconds=args.restart_seconds, progress_seconds=args.progress_seconds,
                                seed=args.seed, thread_env=THREAD_ENV), cases=[], files={})
    for original in [SOURCE, Path(__file__).resolve(), ROOT / "notes/experiments/v047.md",
                     ROOT / "scripts/build_solver.sh", ROOT / "src/bin/v039_exact_board_lns.cpp"]:
        rel = "snapshot/" + original.name
        shutil.copy2(original, run / rel)
        manifest["files"][rel] = dict(source=str(original.relative_to(ROOT)), sha256=sha256(run / rel))
    rejected = []
    source_dirs = sorted(p for p in (ROOT / "results/out").iterdir()
                         if p.is_dir() and re.match(r"v\d{3}_", p.name) and 20 <= int(p.name[1:4]) <= 46)
    for case in args.cases:
        base = run / "cases" / case
        (base / "seeds").mkdir(parents=True)
        original_input = ROOT / "tools/in" / f"{case}.txt"
        problem = Problem.read(original_input)
        shutil.copy2(original_input, base / "input.txt")
        candidates = []
        for source_dir in source_dirs:
            path = source_dir / f"{case}.txt"
            if not path.is_file():
                continue
            try:
                raw = path.read_text()
                text = canonical_plan(raw)
                metrics = problem.replay(text)
                candidates.append(dict(source=str(path.relative_to(ROOT)), source_sha256=sha256(path),
                                       plan_sha256=hashlib.sha256(text.encode()).hexdigest(), metrics=metrics, text=text))
            except (ValueError, OSError) as exc:
                rejected.append(dict(case=case, source=str(path.relative_to(ROOT)), error=str(exc)))
        candidates.sort(key=lambda r: (r["metrics"]["T"], r["source"]))
        selected, seen = [], set()
        for candidate in candidates:
            if candidate["plan_sha256"] in seen:
                continue
            seen.add(candidate["plan_sha256"])
            path = base / "seeds" / f"{len(selected):02d}.txt"
            path.write_text(candidate["text"])
            selected.append({k: v for k, v in candidate.items() if k != "text"} | {"path": str(path.relative_to(run))})
            if len(selected) == 8:
                break
        if not selected:
            raise ValueError(f"{case}: 合法な保存初期解が見つからない")
        atomic_text(base / "best.txt", (run / selected[0]["path"]).read_text())
        manifest["cases"].append(dict(case=case, reason=REASONS.get(case, "ユーザー指定"), features=problem.features(),
                                      input=str((base / "input.txt").relative_to(run)), input_sha256=sha256(base / "input.txt"),
                                      reference_T=selected[0]["metrics"]["T"], seeds=selected))
    manifest["status"] = "prepared"
    write_json(run / "manifest.json", manifest)
    write_json(run / "rejected_seeds.json", rejected)
    write_json(run / "status.json", dict(status="prepared", updated_at=utc_now()))
    print(f"保存先: {run}", flush=True)
    print("ケース: " + ", ".join(f"{c['case']} ({c['reference_T']}手から)" for c in manifest["cases"]), flush=True)
    return manifest


@contextmanager
def evaluation_lock():
    path = ROOT / "results/.eval.lock"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+", encoding="utf-8") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            raise RuntimeError("別の評価・長時間探索が実行中。終了後に実行すること。") from exc
        handle.seek(0)
        handle.truncate()
        handle.write(json.dumps(dict(tool="v047_long_search", pid=os.getpid(), started_at=utc_now())) + "\n")
        handle.flush()
        try:
            yield
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def build(run: Path, manifest: dict) -> Path:
    """凍結したソースを1コンパイラでビルドする。探索は呼ばない。"""
    env = os.environ.copy() | THREAD_ENV
    if sys.platform == "darwin":
        env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
        env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
    compiler = shutil.which(env.get("CXX", "g++-15"))
    if not compiler:
        raise RuntimeError("GCC 15が見つからない。CXXにコンパイラを指定すること。")
    version = subprocess.check_output([compiler, "--version"], text=True, env=env)
    binary = run / "snapshot/v047_long_search"
    command = [compiler, "-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
               "-ftrivial-auto-var-init=zero", "-fopenmp", "-DLOCAL",
               str(run / "snapshot/v047_long_search.cpp"), "-o", str(binary)]
    with (run / "build.log").open("w") as output:
        subprocess.run(command, cwd=ROOT, env=env, stdout=output, stderr=subprocess.STDOUT, check=True)
    manifest["build"] = dict(command=command, compiler_version=version, binary_sha256=sha256(binary),
                             local=True, environment={k: env[k] for k in ("SDKROOT", "MACOSX_DEPLOYMENT_TARGET") if k in env})
    write_json(run / "manifest.json", manifest)
    return binary


class ProcessControl:
    def __init__(self):
        self.stop = threading.Event()
        self.lock = threading.Lock()
        self.processes = {}
        self.max_active = 0

    def register(self, process):
        with self.lock:
            self.processes[process.pid] = process
            self.max_active = max(self.max_active, len(self.processes))
            if len(self.processes) > 2:
                self.stop.set()
                raise RuntimeError("探索プロセス数が2を超えた")

    def unregister(self, process):
        with self.lock:
            self.processes.pop(process.pid, None)

    @staticmethod
    def send(process, signum):
        if process.poll() is None:
            try:
                os.killpg(process.pid, signum)
            except ProcessLookupError:
                pass

    def stop_all(self):
        self.stop.set()
        with self.lock:
            for process in self.processes.values():
                self.send(process, signal.SIGTERM)


def read_events(path: Path, offset: int = 0):
    if not path.exists():
        return [], offset
    events = []
    with path.open("r", encoding="utf-8") as handle:
        handle.seek(offset)
        while True:
            start = handle.tell()
            line = handle.readline()
            if not line:
                return events, handle.tell()
            if not line.endswith("\n"):
                # A live writer or interrupted final write may leave one tail.
                return events, start
            events.append(json.loads(line))


def checked_event_plan(search: Path, event: dict, problem: Problem) -> tuple[Path, dict]:
    plan = (search / event["plan"]).resolve()
    if not plan.is_relative_to(search.resolve()):
        raise ValueError("イベントの操作列パスが保存先の外を指している")
    metrics = problem.replay(plan.read_text())
    if metrics["T"] != event["T"] or event.get("E") != 0:
        raise ValueError("イベントと独立再生のT/Eが一致しない")
    if event.get("before_plan"):
        before = (search / event["before_plan"]).resolve()
        if not before.is_relative_to(search.resolve()):
            raise ValueError("変更前の操作列パスが保存先の外を指している")
        if problem.replay(before.read_text())["T"] != event["before_T"]:
            raise ValueError("変更前の操作列のTが一致しない")
    return plan, metrics


def run_worker(run: Path, binary: Path, case: dict, mode: str, config: dict,
               case_start: float, control: ProcessControl) -> None:
    run = run.resolve()
    worker = run / "cases" / case["case"] / mode
    worker.mkdir()
    initial = run / case["seeds"][0]["path"]
    atomic_text(worker / "best.txt", initial.read_text())
    state = dict(status="running", T=case["reference_T"], rounds=0, updates=0, elapsed_sec=0.0)
    write_json(worker / "status.json", state)
    problem = Problem.read(run / case["input"])
    deadline = case_start + config["minutes_per_case"] * 60
    round_id = 0
    env = os.environ.copy() | THREAD_ENV
    try:
        while not control.stop.is_set():
            remaining = deadline - time.monotonic()
            if remaining <= 0.05 or (mode == "continuous" and round_id):
                break
            if mode == "multistart" and round_id % 2:
                initial = run / case["seeds"][(round_id // 2 + 1) % len(case["seeds"])]["path"]
            else:
                initial = worker / "best.txt"
            folder = worker / f"round_{round_id:04d}"
            folder.mkdir()
            seed_text = initial.read_text()
            atomic_text(folder / "input_plan.txt", seed_text)
            material = f"{config['seed']}:{case['case']}:{mode}:{round_id}".encode()
            seed = int.from_bytes(hashlib.sha256(material).digest()[:8], "big")
            seconds = min(remaining, config["restart_seconds"]) if mode == "multistart" else remaining
            command = [str(binary), "--initial-plan", str(folder / "input_plan.txt"),
                       "--output-dir", str(folder / "search"), "--seconds", f"{seconds:.6f}",
                       "--seed", str(seed), "--progress-seconds", str(config["progress_seconds"])]
            meta = dict(mode=mode, round=round_id, seed=str(seed), command=command, status="running",
                        initial_source=str(initial.relative_to(run)), initial_sha256=sha256(folder / "input_plan.txt"),
                        requested_seconds=seconds, started_offset_sec=time.monotonic() - case_start, started_at=utc_now())
            write_json(folder / "round.json", meta)
            offset = 0
            stop_at = None
            last_record = None
            seen_finish = None
            process = None

            def collect():
                nonlocal offset, last_record, seen_finish
                events, offset = read_events(folder / "search/events.jsonl", offset)
                for event in events:
                    last_record = event
                    if event["type"] == "finish":
                        seen_finish = event
                    if event["type"] not in ("initial", "improvement"):
                        continue
                    path, metrics = checked_event_plan(folder / "search", event, problem)
                    if metrics["T"] < state["T"]:
                        atomic_text(worker / "best.txt", path.read_text())
                        state["T"] = metrics["T"]
                        state["updates"] += 1
                        update = dict(case=case["case"], mode=mode, round=round_id,
                                      elapsed_sec=meta["started_offset_sec"] + event["elapsed_sec"],
                                      T=metrics["T"], plan=str(path.relative_to(run)), reason=event["reason"])
                        with (worker / "improvements.jsonl").open("a") as log:
                            log.write(json.dumps(update, ensure_ascii=False) + "\n")
                state.update(elapsed_sec=time.monotonic() - case_start, round=round_id, last_record=last_record)
                write_json(worker / "status.json", state)

            try:
                with (run / case["input"]).open("rb") as inp, (folder / "stdout.txt").open("wb") as out, (folder / "stderr.log").open("wb") as err:
                    process = subprocess.Popen(command, stdin=inp, stdout=out, stderr=err, env=env, cwd=ROOT, start_new_session=True)
                    control.register(process)
                    while process.poll() is None:
                        collect()
                        now = time.monotonic()
                        if now > deadline + 10 and not control.stop.is_set():
                            raise RuntimeError("ケースの期限を10秒以上超過した")
                        if control.stop.is_set():
                            if stop_at is None:
                                ProcessControl.send(process, signal.SIGTERM)
                                stop_at = now
                            elif now - stop_at > 5:
                                ProcessControl.send(process, signal.SIGKILL)
                        time.sleep(0.25)
                    returncode = process.wait()
                collect()
                meta.update(returncode=returncode, ended_offset_sec=time.monotonic() - case_start, finished_at=utc_now())
                if control.stop.is_set():
                    meta["status"] = "interrupted"
                elif returncode != 0:
                    raise RuntimeError(f"探索が異常終了した: code={returncode}, {folder / 'stderr.log'}")
                elif not seen_finish or seen_finish.get("status") not in ("completed", "time_limit"):
                    raise RuntimeError("正常終了の記録がない")
                else:
                    result = problem.replay((folder / "stdout.txt").read_text())
                    if result["T"] != seen_finish["T"]:
                        raise RuntimeError("最終出力と終了記録が一致しない")
                    meta["status"] = "completed"
                    state["rounds"] += 1
                write_json(folder / "round.json", meta)
            except BaseException as exc:
                if process is not None and process.poll() is None:
                    ProcessControl.send(process, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        ProcessControl.send(process, signal.SIGKILL)
                        process.wait()
                meta.update(status="failed", error=str(exc), ended_offset_sec=time.monotonic() - case_start)
                write_json(folder / "round.json", meta)
                raise
            finally:
                if process is not None:
                    control.unregister(process)
            round_id += 1
        state.update(status="interrupted" if control.stop.is_set() else "completed", elapsed_sec=time.monotonic() - case_start)
        write_json(worker / "status.json", state)
    except BaseException as exc:
        state.update(status="failed", error=str(exc), elapsed_sec=time.monotonic() - case_start)
        write_json(worker / "status.json", state)
        control.stop_all()
        raise


def write_csv(path: Path, rows: list[dict], fields: list[str]) -> None:
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=fields)
    writer.writeheader()
    writer.writerows(rows)
    atomic_text(path, output.getvalue())


def summarize(run: Path) -> list[dict]:
    run = run.resolve()
    manifest = json.loads((run / "manifest.json").read_text())
    summaries, curves, improvements = [], [], []
    duration = manifest["config"]["minutes_per_case"] * 60
    marks = sorted(set([t for t in [2, 10, 30, 60, 120, 300, 600, 900, 1800, 2700, 3600] if t <= duration] + [duration]))
    for case in manifest["cases"]:
        problem = Problem.read(run / case["input"])
        best_text = (run / case["seeds"][0]["path"]).read_text()
        case_best = case["reference_T"]
        for mode in MODES:
            worker = run / "cases" / case["case"] / mode
            state = json.loads((worker / "status.json").read_text()) if (worker / "status.json").exists() else dict(status="not_started", elapsed_sec=0)
            best = case["reference_T"]
            history = [(0.0, best)]
            cpu = 0.0
            peak_rss = 0
            rounds = 0
            observed_until = 0.0
            for folder in sorted(worker.glob("round_*")):
                meta = json.loads((folder / "round.json").read_text())
                events, _ = read_events(folder / "search/events.jsonl")
                if events:
                    cpu += events[-1].get("cpu_sec", 0)
                    observed_until = max(observed_until, meta["started_offset_sec"] + events[-1]["elapsed_sec"])
                rounds += meta.get("status") == "completed"
                for event in events:
                    peak_rss = max(peak_rss, event.get("peak_rss_bytes", 0))
                    if event["type"] not in ("initial", "improvement"):
                        continue
                    path, metrics = checked_event_plan(folder / "search", event, problem)
                    elapsed = meta["started_offset_sec"] + event["elapsed_sec"]
                    history.append((elapsed, metrics["T"]))
                    best = min(best, metrics["T"])
                    if metrics["T"] < case_best:
                        case_best = metrics["T"]
                        best_text = path.read_text()
                    if event["type"] == "improvement":
                        improvements.append(dict(case=case["case"], mode=mode, round=meta["round"], elapsed_sec=round(elapsed, 6),
                                                 T=metrics["T"], saved=event["saved"], reason=event["reason"],
                                                 plan=str(path.relative_to(run)), before_plan=str((folder / "search" / event["before_plan"]).relative_to(run)) if event["before_plan"] else "",
                                                 previous_best_plan=str((folder / "search" / event["previous_best_plan"]).relative_to(run)),
                                                 common_prefix_moves=event["common_prefix_moves"], common_suffix_moves=event["common_suffix_moves"]))
            summaries.append(dict(case=case["case"], mode=mode, status=state["status"], initial_T=case["reference_T"],
                                  best_T=best, saved=case["reference_T"] - best, completed_rounds=rounds,
                                  elapsed_sec=round(state["elapsed_sec"], 3), cpu_sec=round(cpu, 3), peak_rss_bytes=peak_rss))
            for mark in marks:
                if state["status"] != "completed" and mark > observed_until:
                    continue
                at = min(T for t, T in history if t <= mark)
                curves.append(dict(case=case["case"], mode=mode, elapsed_sec=mark, best_T=at, saved=case["reference_T"] - at))
        atomic_text(run / "cases" / case["case"] / "best.txt", best_text)
    write_csv(run / "summary.csv", summaries, ["case", "mode", "status", "initial_T", "best_T", "saved", "completed_rounds", "elapsed_sec", "cpu_sec", "peak_rss_bytes"])
    write_csv(run / "learning_curve.csv", curves, ["case", "mode", "elapsed_sec", "best_T", "saved"])
    write_csv(run / "improvements.csv", improvements, ["case", "mode", "round", "elapsed_sec", "T", "saved", "reason", "plan", "before_plan", "previous_best_plan", "common_prefix_moves", "common_suffix_moves"])
    return summaries


def run_search(run: Path, manifest: dict, binary: Path) -> int:
    run = run.resolve()
    control = ProcessControl()
    status = "completed"
    error = None
    try:
        for case in manifest["cases"]:
            print(f"[{case['case']}] 開始: {manifest['config']['minutes_per_case']:g}分 / 探索2プロセス", flush=True)
            started = time.monotonic()
            write_json(run / "status.json", dict(status="running", case=case["case"], updated_at=utc_now()))
            pool = ThreadPoolExecutor(max_workers=2)
            futures = [pool.submit(run_worker, run, binary, case, mode, manifest["config"], started, control) for mode in MODES]
            try:
                next_print = started + 15
                while not all(f.done() for f in futures):
                    for f in futures:
                        if f.done():
                            f.result()
                    if time.monotonic() >= next_print:
                        values = []
                        for mode in MODES:
                            p = run / "cases" / case["case"] / mode / "status.json"
                            state = json.loads(p.read_text()) if p.exists() else {}
                            values.append(f"{mode}={state.get('T', '?')}手")
                        print(f"[{case['case']}] {(time.monotonic()-started)/60:.1f}分: " + ", ".join(values), flush=True)
                        next_print = time.monotonic() + 60
                    time.sleep(0.25)
                for future in futures:
                    future.result()
            except BaseException:
                control.stop_all()
                raise
            finally:
                pool.shutdown(wait=True)
            rows = summarize(run)
            row = [r for r in rows if r["case"] == case["case"]]
            print(f"[{case['case']}] 完了: " + ", ".join(f"{r['mode']}={r['best_T']}手 (-{r['saved']})" for r in row), flush=True)
    except KeyboardInterrupt:
        status = "interrupted"
        control.stop_all()
        print("停止した。保存済みの最良解と途中集計を保持する。", flush=True)
    except Exception as exc:
        status, error = "failed", str(exc)
        control.stop_all()
        print(f"失敗: {exc}", file=sys.stderr, flush=True)
    finally:
        try:
            summarize(run)
        except Exception as exc:
            status, error = "failed", f"集計失敗: {exc}; 元のエラー: {error}"
        write_json(run / "status.json", dict(status=status, error=error, max_active_search_processes=control.max_active, updated_at=utc_now()))
    return 0 if status == "completed" else 130 if status == "interrupted" else 1


def parse_args(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--minutes-per-case", type=float, default=30, help="1ケースの実時間。既定30、最大60分")
    p.add_argument("--workers", type=int, choices=[2], default=2, help="同時探索数。2に固定")
    p.add_argument("--restart-seconds", type=float, default=60, help="再出発側の1回の探索秒数")
    p.add_argument("--progress-seconds", type=float, default=10, help="C++の定期ログ間隔")
    p.add_argument("--cases", nargs="+", default=DEFAULT_CASES, help="例: --cases 0035 0033 0060")
    p.add_argument("--seed", type=int, default=20260929)
    p.add_argument("--run-dir", type=Path, help="新規保存先。既存ディレクトリは上書きしない")
    p.add_argument("--prepare-only", action="store_true", help="入力と保存解を凍結・検証する。ビルド・探索は行わない")
    p.add_argument("--summarize", type=Path, metavar="RUN_DIR", help="保存済み履歴からCSVとbest.txtを再集計する。探索は行わない")
    args = p.parse_args(argv)
    if not math.isfinite(args.minutes_per_case) or not 0 < args.minutes_per_case <= 60:
        p.error("--minutes-per-case は0より大きく60以下")
    if not math.isfinite(args.restart_seconds) or not 0 < args.restart_seconds <= 3600:
        p.error("--restart-seconds は0より大きく3600以下")
    if not math.isfinite(args.progress_seconds) or args.progress_seconds <= 0:
        p.error("--progress-seconds は正の有限値")
    if not 0 <= args.seed < 2**64:
        p.error("--seed は64 bit符号なし整数")
    if len(set(args.cases)) != len(args.cases) or any(not re.fullmatch(r"00\d\d", c) or c == "0000" for c in args.cases):
        p.error("--cases は重複しない0001〜0099")
    return args


def main(argv=None) -> int:
    args = parse_args(argv)
    if args.summarize:
        run = args.summarize.resolve()
        rows = summarize(run)
        print(f"{len(rows)}行を再集計: {run / 'summary.csv'}")
        return 0
    run_id = datetime.now().strftime("%Y%m%dT%H%M%S") + "_" + uuid.uuid4().hex[:8]
    run = (args.run_dir or ROOT / "results/long_search/v047" / run_id).resolve()
    with evaluation_lock():
        manifest = prepare(args, run)
        if args.prepare_only:
            summarize(run)
            print("準備と保存解の検証が完了。探索は未実行。")
            return 0
        try:
            binary = build(run, manifest)
        except BaseException as exc:
            write_json(run / "status.json", dict(status="build_failed", error=str(exc), updated_at=utc_now()))
            raise
        print(f"ビルド完了。予定時間は約{len(args.cases)*args.minutes_per_case/60:g}時間。Ctrl+Cで保存して停止。", flush=True)
        return run_search(run, manifest, binary)


def request_interrupt(signum, frame):
    # The children have their own process groups; let run_search stop them.
    raise KeyboardInterrupt


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, request_interrupt)
    if hasattr(signal, "SIGHUP"):
        signal.signal(signal.SIGHUP, request_interrupt)
    try:
        sys.exit(main())
    except (ValueError, OSError, RuntimeError, subprocess.CalledProcessError) as exc:
        print(f"v047: {exc}", file=sys.stderr)
        sys.exit(1)
