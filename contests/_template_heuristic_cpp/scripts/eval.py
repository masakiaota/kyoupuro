#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import fcntl
import json
import math
import os
import re
import secrets
import signal
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
from typing import Iterator, Optional

from local_config import LocalProfile, describe_profile, load_profile
from eval_records import records_path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent
TOOLS_MANIFEST = ROOT_DIR / "tools" / "Cargo.toml"
BUILD_SOLVER = SCRIPT_DIR / "build_solver.sh"
SOLVER_BIN_DIR = ROOT_DIR / "target" / "release"
TOOLS_BIN_DIR = ROOT_DIR / "tools" / "target" / "release"
DEFAULT_INPUT_DIR = ROOT_DIR / "tools" / "in"
SUMMARY_CSV = ROOT_DIR / "results" / "score_summary.csv"
DETAIL_CSV = ROOT_DIR / "results" / "score_detail.csv"
EVAL_LOCK = ROOT_DIR / "results" / ".eval.lock"
LOCK_BUSY_EXIT_CODE = 75
# 問題文の実行時間制限 (秒)。コンテストごとに合わせる。
PROBLEM_TIME_LIMIT_SEC = 2.0
# 時間切れは solver が報告する process CPU 時間で判定する。実時間の打ち切りは、並列評価で CPU を
# 待つぶんの延長を許し、ハングだけを止めるための上限であり、時間切れ判定の制限時間の 3 倍とする。
HANG_GUARD_FACTOR = 3.0
# v0000_template.cpp の TimeKeeper が終了時に stderr へ出す行。
SOLVER_TIME_PATTERN = re.compile(rb"^time_keeper: solver_ms=(\d+) wait_ms=(\d+)$", re.MULTILINE)

SUMMARY_HEADER = [
    "bin",
    "total_avg",
    "total_sum",
    "total_min",
    "total_max",
    "avg_elapsed",
    "max_elapsed",
    "eval_set",
    "total_cases",
    "label",
    "executed_at",
]


@dataclass(frozen=True)
class CaseResult:
    case_name: str
    status: str
    score: Optional[int]
    elapsed: int
    stdout_path: str
    # "solver": TimeKeeper が報告した計算時間。"wall": 報告のない solver の実時間。
    elapsed_source: str = "wall"


def eprint(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="./scripts/eval.py",
        description=(
            "Build solver and scorer once, warm up with one solver -> score run, "
            "then run all solvers before scoring their outputs. "
            "Elapsed metrics and time limits use the process CPU time reported by TimeKeeper; "
            "solvers without the report fall back to wall time."
        ),
    )
    parser.add_argument("bin_name", help="C++ solver name under src/bin or adhoc/bin")
    parser.add_argument(
        "input_dir",
        nargs="?",
        default=str(DEFAULT_INPUT_DIR),
        help="Input directory to evaluate (default: tools/in)",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show per-case progress logs",
    )
    parser.add_argument(
        "-j",
        "--jobs",
        type=int,
        default=None,
        help="Parallel jobs (default: local profile, or 1 if unavailable)",
    )
    parser.add_argument("--profile", help="Select a local profile instead of matching the hostname")
    parser.add_argument(
        "--time-limit-sec",
        type=float,
        default=None,
        help=(
            "Measured per-case time limit in seconds, without LOCAL scaling; "
            "does not change solver budgets "
            "(default: PROBLEM_TIME_LIMIT_SEC scaled by the local profile)"
        ),
    )
    parser.add_argument(
        "--label",
        default="",
        help="Optional experiment label recorded in CSV/JSONL",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Do not write score_summary.csv, score_detail.csv, or dataset evaluation records",
    )
    parser.add_argument(
        "--no-local",
        action="store_true",
        help="Build the solver without the LOCAL macro for production-like behavior",
    )
    parser.add_argument(
        "--wait-lock",
        action="store_true",
        help="Wait for another eval.py to finish instead of exiting immediately",
    )
    args = parser.parse_args()
    if args.jobs is not None and args.jobs < 1:
        parser.error("jobs must be >= 1")
    if args.time_limit_sec is not None and (
        not math.isfinite(args.time_limit_sec) or args.time_limit_sec <= 0
    ):
        parser.error("time-limit-sec must be finite and > 0")
    return args


def normalize_dir(path: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(ROOT_DIR.resolve()).as_posix()
    except ValueError:
        return resolved.as_posix()


def round_half_up(numerator: int, denominator: int) -> int:
    return int(
        (Decimal(numerator) / Decimal(denominator)).quantize(
            Decimal("1"), rounding=ROUND_HALF_UP
        )
    )


def resolve_solver_source(bin_name: str) -> Path:
    """solver またはローカル補助 bin の C++ ソースを解決する。"""
    solver_src = ROOT_DIR / "src" / "bin" / f"{bin_name}.cpp"
    if solver_src.is_file():
        return solver_src
    adhoc_src = ROOT_DIR / "adhoc" / "bin" / f"{bin_name}.cpp"
    if adhoc_src.is_file():
        return adhoc_src
    raise SystemExit(f"error: not found: {solver_src} nor {adhoc_src}")


def ensure_tools_ready() -> None:
    if not TOOLS_MANIFEST.is_file():
        raise SystemExit(f"error: tools manifest not found: {TOOLS_MANIFEST}")


def build_solver(bin_name: str, *, local_enabled: bool, local_time_ratio: float) -> None:
    command = [str(BUILD_SOLVER)]
    if not local_enabled:
        command.append("--no-local")
    else:
        command.extend(["--time-ratio", repr(local_time_ratio)])
    command.append(bin_name)
    result = subprocess.run(command, cwd=ROOT_DIR, stdout=subprocess.DEVNULL)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def build_tool(bin_name: str) -> None:
    command = [
        "cargo",
        "build",
        "--release",
        "--quiet",
        "--manifest-path",
        str(TOOLS_MANIFEST),
        "--bin",
        bin_name,
    ]
    result = subprocess.run(command, cwd=ROOT_DIR)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def list_input_files(input_dir: Path) -> list[Path]:
    files = sorted(path for path in input_dir.rglob("*") if path.is_file())
    if not files:
        raise SystemExit(f"error: input directory is empty: {input_dir}")
    return files


def ensure_unique_basenames(paths: list[Path]) -> None:
    seen: dict[str, Path] = {}
    duplicates: set[str] = set()
    for path in paths:
        base = path.name
        if base in seen:
            duplicates.add(base)
        else:
            seen[base] = path
    if duplicates:
        duplicate_list = "\n".join(sorted(duplicates))
        raise SystemExit(
            "error: input directory contains duplicate basenames; results would collide\n"
            f"files with duplicated basename:\n{duplicate_list}"
        )


def ensure_csv_header(path: Path, header: list[str]) -> None:
    expected = ",".join(header)
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        with path.open("r", encoding="utf-8", newline="") as handle:
            first_line = handle.readline().rstrip("\r\n")
        if first_line == "":
            with path.open("w", encoding="utf-8", newline="") as handle:
                handle.write(expected + "\n")
            return
        if first_line != expected:
            raise SystemExit(
                f"error: CSV header mismatch: {path}\n"
                f"expected: {expected}\n"
                f"actual:   {first_line}"
            )
        return
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(expected + "\n")


def append_csv_row(path: Path, row: list[str | int]) -> None:
    with path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(row)


def append_jsonl(path: Path, records: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")))
            handle.write("\n")


def make_lock_record(args: argparse.Namespace, normalized_input_dir: str) -> str:
    record = {
        "pid": os.getpid(),
        "bin": args.bin_name,
        "jobs": args.jobs,
        "input_dir": normalized_input_dir,
        "started_at": datetime.now().astimezone().isoformat(timespec="seconds"),
    }
    return json.dumps(record, ensure_ascii=False, separators=(",", ":"))


@contextmanager
def acquire_eval_lock(
    args: argparse.Namespace,
    normalized_input_dir: str,
) -> Iterator[None]:
    EVAL_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with EVAL_LOCK.open("a+", encoding="utf-8") as handle:
        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            handle.seek(0)
            owner = handle.read().strip()
            if not args.wait_lock:
                eprint("eval: another eval.py is already running.")
                if owner:
                    eprint(f"eval: running eval: {owner}")
                eprint(
                    "eval: exit without running. "
                    "Use --wait-lock to wait for the current eval to finish."
                )
                raise SystemExit(LOCK_BUSY_EXIT_CODE)

            eprint("eval: another eval.py is already running; waiting...")
            if owner:
                eprint(f"eval: running eval: {owner}")
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)

        handle.seek(0)
        handle.truncate()
        handle.write(make_lock_record(args, normalized_input_dir) + "\n")
        handle.flush()
        os.fsync(handle.fileno())

        try:
            yield
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def compute_detail_header() -> list[str]:
    tools_inputs = list_input_files(DEFAULT_INPUT_DIR)
    ensure_unique_basenames(tools_inputs)
    case_columns = sorted(path.name for path in tools_inputs)
    return ["bin", "total_avg", "max_elapsed", *case_columns, "label", "executed_at"]


def clean_output_dir(output_dir: Path) -> None:
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)


def parse_score(stdout: str) -> Optional[int]:
    tokens = stdout.split()
    if not tokens:
        return None
    try:
        return int(tokens[-1])
    except ValueError:
        return None


def persist_capture(path: Path, content: bytes) -> None:
    # Truncated workspace captures were observed after normal child exits.
    # Publish complete bytes after exit and verify them before later scoring.
    temporary = path.with_name(path.name + ".capture_partial")
    with temporary.open("wb") as handle:
        handle.write(content)
        handle.flush()
        os.fsync(handle.fileno())
    temporary.replace(path)
    if path.read_bytes() != content:
        raise OSError(f"capture verification failed: {path}")


def run_solver(
    case_path: Path,
    solver_bin: Path,
    output_dir: Path,
    verbose: bool,
    timeout_sec: float,
) -> CaseResult:
    case_name = case_path.name
    output_path = output_dir / case_name
    err_path = output_dir / f"{case_name}.err"
    stdout_path = output_path.relative_to(ROOT_DIR).as_posix()

    if verbose:
        eprint(f"start: {case_name}")

    hang_guard_sec = timeout_sec * HANG_GUARD_FACTOR
    run_start_ns = time.monotonic_ns()
    try:
        with case_path.open("rb") as fin:
            # インタラクティブ問題では公式 tester 経由の起動に書き換える。
            # 独立した session で起動し、ハング時は子プロセスごとまとめて止める。
            process = subprocess.Popen(
                [str(solver_bin)],
                stdin=fin,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
            try:
                captured_stdout, captured_stderr = process.communicate(timeout=hang_guard_sec)
                returncode = process.returncode
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                captured_stdout, captured_stderr = process.communicate()
                captured_stderr += f"eval: wall-clock hang guard exceeded ({hang_guard_sec:g}s)\n".encode()
                persist_capture(output_path, captured_stdout)
                persist_capture(err_path, captured_stderr)
                run_elapsed = (time.monotonic_ns() - run_start_ns) // 1_000_000
                return CaseResult(case_name, "timeout", None, int(run_elapsed), stdout_path)
            persist_capture(output_path, captured_stdout)
            persist_capture(err_path, captured_stderr)
    except OSError:
        run_elapsed = (time.monotonic_ns() - run_start_ns) // 1_000_000
        return CaseResult(
            case_name=case_name,
            status="run_fail",
            score=None,
            elapsed=int(run_elapsed),
            stdout_path=stdout_path,
        )
    run_elapsed = int((time.monotonic_ns() - run_start_ns) // 1_000_000)
    elapsed, elapsed_source = run_elapsed, "wall"
    reports = SOLVER_TIME_PATTERN.findall(err_path.read_bytes())
    if reports:
        elapsed, elapsed_source = int(reports[-1][0]), "solver"
    if returncode != 0:
        if verbose:
            eprint(f"fail(run): {case_name}")
        return CaseResult(
            case_name=case_name,
            status="run_fail",
            score=None,
            elapsed=elapsed,
            stdout_path=stdout_path,
            elapsed_source=elapsed_source,
        )
    if elapsed > timeout_sec * 1000:
        with err_path.open("ab") as ferr:
            ferr.write(f"eval: time limit exceeded ({elapsed_source}={elapsed}ms)\n".encode())
        return CaseResult(case_name, "timeout", None, elapsed, stdout_path, elapsed_source)

    return CaseResult(case_name, "run_ok", None, elapsed, stdout_path, elapsed_source)


def score_case(
    case_path: Path,
    run: CaseResult,
    score_bin: Path,
    output_dir: Path,
    verbose: bool,
) -> CaseResult:
    if run.status != "run_ok":
        return run
    case_name = run.case_name
    output_path = output_dir / case_name
    err_path = output_dir / f"{case_name}.err"
    stdout_path = run.stdout_path
    run_elapsed = run.elapsed
    try:
        with err_path.open("ab") as ferr:
            score_result = subprocess.run(
                [str(score_bin), str(case_path), str(output_path)],
                stdout=subprocess.PIPE,
                stderr=ferr,
                text=True,
            )
    except OSError:
        return CaseResult(
            case_name=case_name,
            status="score_fail",
            score=None,
            elapsed=run_elapsed,
            stdout_path=stdout_path,
            elapsed_source=run.elapsed_source,
        )
    if score_result.returncode != 0:
        if verbose:
            eprint(f"fail(score): {case_name}")
        return CaseResult(
            case_name=case_name,
            status="score_fail",
            score=None,
            elapsed=run_elapsed,
            stdout_path=stdout_path,
            elapsed_source=run.elapsed_source,
        )

    score = parse_score(score_result.stdout)
    if score is None:
        if verbose:
            eprint(f"fail(parse): {case_name}")
        return CaseResult(
            case_name=case_name,
            status="score_parse_fail",
            score=None,
            elapsed=run_elapsed,
            stdout_path=stdout_path,
            elapsed_source=run.elapsed_source,
        )

    if verbose:
        eprint(
            f"done: {case_name} score={score} elapsed={run_elapsed}ms output={stdout_path}"
        )
    return CaseResult(
        case_name=case_name,
        status="ok",
        score=score,
        elapsed=run_elapsed,
        stdout_path=stdout_path,
        elapsed_source=run.elapsed_source,
    )


def run_case(
    case_path: Path,
    solver_bin: Path,
    score_bin: Path,
    output_dir: Path,
    verbose: bool,
    timeout_sec: float,
) -> CaseResult:
    run = run_solver(case_path, solver_bin, output_dir, verbose, timeout_sec)
    return score_case(case_path, run, score_bin, output_dir, verbose)


def evaluate_cases(
    input_files: list[Path],
    solver_bin: Path,
    score_bin: Path,
    output_dir: Path,
    jobs: int,
    verbose: bool,
    timeout_sec: float,
) -> list[CaseResult]:
    results_by_name: dict[str, CaseResult] = {}
    with ThreadPoolExecutor(max_workers=jobs) as executor:
        future_map = {
            executor.submit(run_solver, path, solver_bin, output_dir, verbose, timeout_sec): path
            for path in input_files
        }
        for future in as_completed(future_map):
            result = future.result()
            results_by_name[result.case_name] = result
        # 採点処理の CPU / メモリ負荷が、設定した solver の並列実行に混ざらないようにする。
        score_futures = [
            executor.submit(score_case, path, results_by_name[path.name], score_bin, output_dir, verbose)
            for path in input_files
        ]
        for future in as_completed(score_futures):
            result = future.result()
            results_by_name[result.case_name] = result
    return [results_by_name[path.name] for path in input_files]


def warm_up_case(
    case_path: Path,
    solver_bin: Path,
    score_bin: Path,
    output_dir: Path,
    verbose: bool,
    timeout_sec: float,
) -> None:
    if verbose:
        eprint(f"warmup: start case={case_path.name}")

    try:
        with tempfile.TemporaryDirectory(prefix=".warmup_", dir=output_dir) as temp_dir:
            result = run_case(
                case_path=case_path,
                solver_bin=solver_bin,
                score_bin=score_bin,
                output_dir=Path(temp_dir),
                verbose=False,
                timeout_sec=timeout_sec,
            )
    except OSError as error:
        if verbose:
            eprint(f"warmup: done case={case_path.name} status=setup_fail error={error}")
        return

    if verbose:
        score = "" if result.score is None else f" score={result.score}"
        eprint(
            "warmup: "
            f"done case={result.case_name} status={result.status}{score} "
            f"elapsed={result.elapsed}ms output=discarded"
        )


def summarize(results: list[CaseResult]) -> tuple[int, int, int, int, int, int]:
    success_results = [result for result in results if result.status == "ok" and result.score is not None]
    if not success_results:
        return (0, 0, 0, 0, 0, 0)

    total_sum = sum(result.score for result in success_results if result.score is not None)
    total_min = min(result.score for result in success_results if result.score is not None)
    total_max = max(result.score for result in success_results if result.score is not None)
    max_elapsed = max(result.elapsed for result in success_results)
    total_avg = round_half_up(total_sum, len(success_results))
    avg_elapsed = round_half_up(
        sum(result.elapsed for result in success_results),
        len(success_results),
    )
    return (total_avg, total_sum, total_min, total_max, avg_elapsed, max_elapsed)


def make_run_id(executed_dt: datetime, bin_name: str) -> str:
    timestamp = executed_dt.strftime("%Y%m%dT%H%M%S%z")
    return f"{timestamp}_{bin_name}_{secrets.token_hex(3)}"


def make_records(
    results: list[CaseResult],
    run_id: str,
    executed_at: str,
    bin_name: str,
    label: str,
    normalized_input_dir: str,
    local_enabled: bool,
    profile: LocalProfile,
    jobs: int,
    time_limit_ms: int,
) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for result in results:
        records.append(
            {
                "run_id": run_id,
                "executed_at": executed_at,
                "bin": bin_name,
                "label": label,
                "local": local_enabled,
                "profile": profile.name,
                "hostname": profile.hostname,
                "local_time_ratio": profile.local_time_ratio if local_enabled else 1.0,
                "jobs": jobs,
                "config_default_reason": profile.reason,
                "input_dir": normalized_input_dir,
                "case_name": result.case_name,
                "score": result.score if result.status == "ok" else None,
                "elapsed": result.elapsed,
                "elapsed_source": result.elapsed_source,
                "time_limit_ms": time_limit_ms,
                "status": result.status,
                "stdout_path": result.stdout_path,
            }
        )
    return records


def run_eval_locked(
    args: argparse.Namespace,
    input_files: list[Path],
    normalized_input_dir: str,
    is_tools_in: bool,
    local_enabled: bool,
    profile: LocalProfile,
) -> int:
    if not args.dry_run:
        ensure_csv_header(SUMMARY_CSV, SUMMARY_HEADER)
        if is_tools_in:
            ensure_csv_header(DETAIL_CSV, compute_detail_header())

    output_dir = ROOT_DIR / "results" / "out" / args.bin_name
    clean_output_dir(output_dir)

    if args.verbose:
        eprint(
            f"eval: bin={args.bin_name} input_dir={normalized_input_dir} "
            f"local={'on' if local_enabled else 'off'} "
            f"parallel={args.jobs} output={output_dir}"
        )

    build_solver(args.bin_name, local_enabled=local_enabled, local_time_ratio=profile.local_time_ratio)
    build_tool("score")

    solver_bin = SOLVER_BIN_DIR / args.bin_name
    score_bin = TOOLS_BIN_DIR / "score"
    if not solver_bin.is_file():
        raise SystemExit(f"error: solver binary not found: {solver_bin}")
    if not score_bin.is_file():
        raise SystemExit(f"error: score binary not found: {score_bin}")

    executed_dt = datetime.now().astimezone()
    executed_at = executed_dt.isoformat(timespec="seconds")
    run_id = make_run_id(executed_dt, args.bin_name)
    # 明示した上限は計測値そのものへ適用する。探索用のLOCAL倍率とは分けて扱う。
    timeout_sec = (
        args.time_limit_sec
        if args.time_limit_sec is not None
        else PROBLEM_TIME_LIMIT_SEC * (profile.local_time_ratio if local_enabled else 1.0)
    )
    eprint(f"eval: time_limit={timeout_sec:g}s hang_guard={timeout_sec * HANG_GUARD_FACTOR:g}s")

    warm_up_case(
        case_path=input_files[0],
        solver_bin=solver_bin,
        score_bin=score_bin,
        output_dir=output_dir,
        verbose=args.verbose,
        timeout_sec=timeout_sec,
    )

    results = evaluate_cases(
        input_files=input_files,
        solver_bin=solver_bin,
        score_bin=score_bin,
        output_dir=output_dir,
        jobs=args.jobs,
        verbose=args.verbose,
        timeout_sec=timeout_sec,
    )

    success_count = sum(result.status == "ok" for result in results)
    failure_count = len(results) - success_count
    total_avg, total_sum, total_min, total_max, avg_elapsed, max_elapsed = summarize(results)

    eprint(
        "eval: "
        f"bin={args.bin_name} eval_set={normalized_input_dir} "
        f"success={success_count} failure={failure_count} "
        f"total_avg={total_avg} avg_elapsed={avg_elapsed} max_elapsed={max_elapsed} "
        f"total_sum={total_sum} total_min={total_min} total_max={total_max} "
        f"total_cases={len(results)} output={output_dir}"
    )

    if not args.dry_run:
        record_file = records_path(normalized_input_dir)
        append_jsonl(
            record_file,
            make_records(
                results=results,
                run_id=run_id,
                executed_at=executed_at,
                bin_name=args.bin_name,
                label=args.label,
                normalized_input_dir=normalized_input_dir,
                local_enabled=local_enabled,
                profile=profile,
                jobs=args.jobs,
                time_limit_ms=round(timeout_sec * 1000),
            ),
        )

        if failure_count == 0:
            append_csv_row(
                SUMMARY_CSV,
                [
                    args.bin_name,
                    total_avg,
                    total_sum,
                    total_min,
                    total_max,
                    avg_elapsed,
                    max_elapsed,
                    normalized_input_dir,
                    len(results),
                    args.label,
                    executed_at,
                ],
            )
            if is_tools_in:
                score_by_case = {result.case_name: result.score for result in results}
                detail_header = compute_detail_header()
                case_columns = detail_header[3:-2]
                detail_row: list[str | int] = [args.bin_name, total_avg, max_elapsed]
                for case_name in case_columns:
                    score = score_by_case.get(case_name)
                    detail_row.append("" if score is None else score)
                detail_row.extend([args.label, executed_at])
                append_csv_row(DETAIL_CSV, detail_row)

    if failure_count != 0:
        return 1
    return 0


def main() -> int:
    args = parse_args()
    profile = load_profile(args.profile)
    eprint(describe_profile(profile))
    if args.jobs is None:
        args.jobs = profile.eval_jobs
    elif args.jobs != profile.eval_jobs:
        eprint(f"eval: jobs={args.jobs} overrides profile jobs={profile.eval_jobs}; calibration conditions differ")
    effective_ratio = 1.0 if args.no_local else profile.local_time_ratio
    eprint(f"eval: local={'off' if args.no_local else 'on'} ratio={effective_ratio:g} parallel={args.jobs}")
    resolve_solver_source(args.bin_name)
    ensure_tools_ready()

    input_dir = Path(args.input_dir).resolve()
    if not input_dir.is_dir():
        raise SystemExit(f"error: input directory not found: {args.input_dir}")

    input_files = list_input_files(input_dir)
    ensure_unique_basenames(input_files)

    normalized_input_dir = normalize_dir(input_dir)
    is_tools_in = input_dir == DEFAULT_INPUT_DIR.resolve()
    local_enabled = not args.no_local

    with acquire_eval_lock(args, normalized_input_dir):
        return run_eval_locked(
            args=args,
            input_files=input_files,
            normalized_input_dir=normalized_input_dir,
            is_tools_in=is_tools_in,
            local_enabled=local_enabled,
            profile=profile,
        )


if __name__ == "__main__":
    raise SystemExit(main())
