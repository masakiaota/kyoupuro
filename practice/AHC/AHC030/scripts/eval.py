#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import select
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
from typing import Optional


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent
SOLVER_MANIFEST = ROOT_DIR / "Cargo.toml"
SOLVER_BIN_DIR = ROOT_DIR / "target" / "release"
DEFAULT_INPUT_DIR = ROOT_DIR / "tools" / "in"
SUMMARY_CSV = ROOT_DIR / "results" / "score_summary.csv"
DETAIL_CSV = ROOT_DIR / "results" / "score_detail.csv"
RECORDS_JSONL = ROOT_DIR / "results" / "eval_records.jsonl"
DEFAULT_TIMEOUT_MS = 120_000

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


def eprint(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def default_jobs() -> int:
    cpu_count = os.cpu_count() or 2
    return max(1, (cpu_count // 2) - 1)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="./scripts/eval.py",
        description=(
            "Build solver once, warm up with one interactive judge run, "
            "then evaluate solver with the AHC030 local judge per case."
        ),
    )
    parser.add_argument("bin_name", help="Rust solver bin name under src/bin")
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
        default=default_jobs(),
        help="Parallel jobs (default: max(1, cpu//2 - 1))",
    )
    parser.add_argument(
        "--label",
        default="",
        help="Optional experiment label recorded in CSV/JSONL",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Do not write score_summary.csv, score_detail.csv, or eval_records.jsonl",
    )
    parser.add_argument(
        "--timeout-ms",
        type=int,
        default=DEFAULT_TIMEOUT_MS,
        help=(
            "Per-case solver-only elapsed timeout in milliseconds "
            f"(default: {DEFAULT_TIMEOUT_MS}; 0 disables timeout)"
        ),
    )
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("jobs must be >= 1")
    if args.timeout_ms < 0:
        parser.error("timeout-ms must be >= 0")
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


def ensure_solver_exists(bin_name: str) -> None:
    solver_src = ROOT_DIR / "src" / "bin" / f"{bin_name}.rs"
    if not solver_src.is_file():
        raise SystemExit(f"error: not found: {solver_src}")


def build_binary(manifest_path: Path, bin_name: str) -> None:
    command = [
        "cargo",
        "build",
        "--release",
        "--quiet",
        "--manifest-path",
        str(manifest_path),
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
        writer = csv.writer(handle)
        writer.writerow(row)


def append_jsonl(path: Path, records: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")))
            handle.write("\n")


def compute_detail_header() -> list[str]:
    tools_inputs = list_input_files(DEFAULT_INPUT_DIR)
    ensure_unique_basenames(tools_inputs)
    case_columns = sorted(path.name for path in tools_inputs)
    return ["bin", "total_avg", "max_elapsed", *case_columns, "label", "executed_at"]


def clean_output_dir(output_dir: Path) -> None:
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)


@dataclass(frozen=True)
class InputData:
    n: int
    m: int
    eps: float
    shapes: list[list[tuple[int, int]]]
    placements: list[tuple[int, int]]
    answer: list[list[int]]
    noise: list[float]


class JudgeError(Exception):
    def __init__(self, status: str, message: str) -> None:
        super().__init__(message)
        self.status = status


class LocalJudge:
    def __init__(self, input_data: InputData) -> None:
        self.input = input_data
        self.responses: list[int] = []
        self.cost = 0.0
        self.finished = False
        self.oil_cell_count = sum(
            1 for row in input_data.answer for value in row if value > 0
        )

    def query_mining(self, point: tuple[int, int]) -> int:
        i, j = point
        self.cost += 1.0
        return self.input.answer[i][j]

    def query_survey(self, points: list[tuple[int, int]]) -> int:
        self.cost += 1.0 / math.sqrt(len(points))
        oil_sum = sum(self.input.answer[i][j] for i, j in points)
        k = float(len(points))
        mu = (k - float(oil_sum)) * self.input.eps + float(oil_sum) * (1.0 - self.input.eps)
        sigma = math.sqrt(k * self.input.eps * (1.0 - self.input.eps))
        noise = self.input.noise[len(self.responses)]
        return max(0, rust_round_to_int(mu + noise * sigma))

    def query_answer(self, points: list[tuple[int, int]]) -> int:
        if (
            len(points) == self.oil_cell_count
            and all(self.input.answer[i][j] > 0 for i, j in points)
        ):
            self.finished = True
            return 1
        self.cost += 1.0
        return 0

    def push_response(self, response: int) -> None:
        self.responses.append(response)

    def score(self) -> int:
        cost = self.cost if self.finished else 1000.0
        return int(math.floor(1_000_000.0 * max(cost, 1.0 / self.input.n) + 0.5))


def rust_round_to_int(value: float) -> int:
    if value >= 0.0:
        return int(math.floor(value + 0.5))
    return int(math.ceil(value - 0.5))


def parse_input_file(path: Path) -> InputData:
    tokens = path.read_text(encoding="utf-8").split()
    pos = 0

    def next_token(name: str) -> str:
        nonlocal pos
        if pos >= len(tokens):
            raise JudgeError("input_parse_fail", f"Unexpected EOF while reading {name}")
        token = tokens[pos]
        pos += 1
        return token

    try:
        n = int(next_token("N"))
        m = int(next_token("M"))
        eps = float(next_token("eps"))
        shapes: list[list[tuple[int, int]]] = []
        for oil_id in range(m):
            d = int(next_token(f"shape {oil_id} size"))
            shape = []
            for _ in range(d):
                i = int(next_token("shape i"))
                j = int(next_token("shape j"))
                shape.append((i, j))
            shapes.append(shape)

        placements = []
        for _ in range(m):
            i = int(next_token("placement i"))
            j = int(next_token("placement j"))
            placements.append((i, j))

        answer = []
        for _ in range(n):
            row = []
            for _ in range(n):
                row.append(int(next_token("answer grid")))
            answer.append(row)

        noise = [float(next_token("noise")) for _ in range(2 * n * n)]
    except ValueError as error:
        raise JudgeError("input_parse_fail", f"Input parse error: {error}") from error

    return InputData(n=n, m=m, eps=eps, shapes=shapes, placements=placements, answer=answer, noise=noise)


def make_initial_input(input_data: InputData) -> bytes:
    lines = [f"{input_data.n} {input_data.m} {input_data.eps:.2f}"]
    for shape in input_data.shapes:
        cells = " ".join(f"{i} {j}" for i, j in shape)
        lines.append(f"{len(shape)} {cells}")
    return ("\n".join(lines) + "\n").encode()


def read_int_token(token: Optional[str], lb: int, ub: int, label: str) -> int:
    if token is None:
        raise JudgeError("invalid_output", f"Unexpected EOF while reading {label}")
    try:
        value = int(token)
    except ValueError as error:
        raise JudgeError("invalid_output", f"Parse error in {label}: {token}") from error
    if value < lb or ub < value:
        raise JudgeError("invalid_output", f"{label} is out of range: {value}")
    return value


def parse_points(tokens: list[str], n: int, count: int) -> list[tuple[int, int]]:
    if len(tokens) != 2 * count:
        raise JudgeError("invalid_output", "Invalid query format")
    points = []
    for idx in range(count):
        i = read_int_token(tokens[2 * idx], 0, n - 1, "i")
        j = read_int_token(tokens[2 * idx + 1], 0, n - 1, "j")
        points.append((i, j))
    if len(set(points)) != count:
        raise JudgeError("invalid_output", "Query contains the same square multiple times.")
    return points


def write_solver_stdin(proc: subprocess.Popen[bytes], data: bytes) -> None:
    if proc.stdin is None:
        raise JudgeError("run_fail", "solver stdin is unavailable")
    try:
        proc.stdin.write(data)
        proc.stdin.flush()
    except BrokenPipeError as error:
        raise JudgeError("run_fail", "solver stdin was closed") from error


def close_solver_stdin(proc: subprocess.Popen[bytes]) -> None:
    if proc.stdin is None or proc.stdin.closed:
        return
    try:
        proc.stdin.close()
    except BrokenPipeError:
        pass


def kill_solver(proc: subprocess.Popen[bytes]) -> None:
    if proc.poll() is not None:
        return
    proc.kill()
    try:
        proc.wait(timeout=1)
    except subprocess.TimeoutExpired:
        pass


def remaining_timeout_sec(elapsed_ns: int, timeout_ms: int) -> Optional[float]:
    if timeout_ms == 0:
        return None
    remaining_ns = timeout_ms * 1_000_000 - elapsed_ns
    return max(0.0, remaining_ns / 1_000_000_000.0)


def read_solver_line(
    proc: subprocess.Popen[bytes],
    timeout_ms: int,
    elapsed_ns: int,
) -> tuple[Optional[bytes], int]:
    if proc.stdout is None:
        raise JudgeError("run_fail", "solver stdout is unavailable")

    start_ns = time.monotonic_ns()
    timeout = remaining_timeout_sec(elapsed_ns, timeout_ms)
    ready, _, _ = select.select([proc.stdout], [], [], timeout)
    if not ready:
        return None, elapsed_ns + (time.monotonic_ns() - start_ns)

    line = proc.stdout.readline()
    return line, elapsed_ns + (time.monotonic_ns() - start_ns)


def wait_solver_exit(
    proc: subprocess.Popen[bytes],
    timeout_ms: int,
    elapsed_ns: int,
) -> tuple[bool, int]:
    close_solver_stdin(proc)
    start_ns = time.monotonic_ns()
    timeout = remaining_timeout_sec(elapsed_ns, timeout_ms)
    try:
        proc.wait(timeout=timeout)
        return True, elapsed_ns + (time.monotonic_ns() - start_ns)
    except subprocess.TimeoutExpired:
        return False, elapsed_ns + (time.monotonic_ns() - start_ns)


def run_case(
    case_path: Path,
    solver_bin: Path,
    output_dir: Path,
    verbose: bool,
    timeout_ms: int,
) -> CaseResult:
    case_name = case_path.name
    output_path = output_dir / case_name
    err_path = output_dir / f"{case_name}.err"
    stdout_path = output_path.relative_to(ROOT_DIR).as_posix()

    if verbose:
        eprint(f"start: {case_name}")

    elapsed_ns = 0
    try:
        input_data = parse_input_file(case_path)
        judge = LocalJudge(input_data)
    except JudgeError as error:
        err_path.write_text(f"{error}\n", encoding="utf-8")
        return CaseResult(
            case_name=case_name,
            status=error.status,
            score=None,
            elapsed=0,
            stdout_path=stdout_path,
        )

    try:
        with output_path.open("wb") as fout, err_path.open("wb") as ferr:
            proc = subprocess.Popen(
                [str(solver_bin)],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=ferr,
                cwd=ROOT_DIR,
            )
            try:
                write_solver_stdin(proc, make_initial_input(input_data))

                while len(judge.responses) < 2 * input_data.n * input_data.n:
                    line, elapsed_ns = read_solver_line(proc, timeout_ms, elapsed_ns)
                    if line is None:
                        raise JudgeError("run_timeout", f"solver timed out after {timeout_ms}ms")
                    if line == b"":
                        raise JudgeError("run_fail", "Your program has terminated unexpectedly")

                    fout.write(line)
                    fout.flush()

                    stripped = line.decode("utf-8", errors="replace").strip()
                    if not stripped or stripped.startswith("#"):
                        continue

                    parts = stripped.split()
                    if len(parts) < 2:
                        raise JudgeError("invalid_output", f"Invalid query format: {stripped}")

                    ty = parts[0]
                    num = read_int_token(parts[1], 1, input_data.n * input_data.n, "query size")

                    if ty == "a":
                        points = parse_points(parts[2:], input_data.n, num)
                        response = judge.query_answer(points)
                    elif ty == "q":
                        if num == 1:
                            points = parse_points(parts[2:], input_data.n, 1)
                            response = judge.query_mining(points[0])
                        else:
                            points = parse_points(parts[2:], input_data.n, num)
                            response = judge.query_survey(points)
                    else:
                        raise JudgeError("invalid_output", f"Invalid query format: {stripped}")

                    judge.push_response(response)
                    try:
                        write_solver_stdin(proc, f"{response}\n".encode())
                    except JudgeError:
                        if not (response == 1 and ty == "a"):
                            raise
                    if response == 1 and ty == "a":
                        break

                exited, elapsed_ns = wait_solver_exit(proc, timeout_ms, elapsed_ns)
                if not exited:
                    raise JudgeError("run_timeout", f"solver timed out after {timeout_ms}ms")
            except JudgeError:
                kill_solver(proc)
                raise
            except OSError as error:
                kill_solver(proc)
                raise JudgeError("run_fail", str(error)) from error
    except OSError as error:
        err_path.write_text(f"{error}\n", encoding="utf-8")
        elapsed = elapsed_ns // 1_000_000
        return CaseResult(
            case_name=case_name,
            status="run_fail",
            score=None,
            elapsed=int(elapsed),
            stdout_path=stdout_path,
        )
    except JudgeError as error:
        with err_path.open("ab") as ferr:
            ferr.write(f"{error}\n".encode())
        elapsed = elapsed_ns // 1_000_000
        if verbose:
            eprint(f"fail({error.status}): {case_name} elapsed={elapsed}ms")
        return CaseResult(
            case_name=case_name,
            status=error.status,
            score=None,
            elapsed=int(elapsed),
            stdout_path=stdout_path,
        )

    elapsed = elapsed_ns // 1_000_000
    score = judge.score()

    if verbose:
        eprint(
            f"done: {case_name} score={score} elapsed={elapsed}ms output={stdout_path}"
        )
    return CaseResult(
        case_name=case_name,
        status="ok",
        score=score,
        elapsed=int(elapsed),
        stdout_path=stdout_path,
    )


def evaluate_cases(
    input_files: list[Path],
    solver_bin: Path,
    output_dir: Path,
    jobs: int,
    verbose: bool,
    timeout_ms: int,
) -> list[CaseResult]:
    if jobs == 1:
        return [
            run_case(path, solver_bin, output_dir, verbose, timeout_ms)
            for path in input_files
        ]

    results_by_name: dict[str, CaseResult] = {}
    with ThreadPoolExecutor(max_workers=jobs) as executor:
        future_map = {
            executor.submit(
                run_case,
                path,
                solver_bin,
                output_dir,
                verbose,
                timeout_ms,
            ): path
            for path in input_files
        }
        for future in as_completed(future_map):
            result = future.result()
            results_by_name[result.case_name] = result
    return [results_by_name[path.name] for path in input_files]


def warm_up_case(
    case_path: Path,
    solver_bin: Path,
    output_dir: Path,
    verbose: bool,
    timeout_ms: int,
) -> None:
    if verbose:
        eprint(f"warmup: start case={case_path.name}")

    try:
        with tempfile.TemporaryDirectory(prefix=".warmup_", dir=output_dir) as temp_dir:
            result = run_case(
                case_path=case_path,
                solver_bin=solver_bin,
                output_dir=Path(temp_dir),
                verbose=False,
                timeout_ms=timeout_ms,
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
) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for result in results:
        records.append(
            {
                "run_id": run_id,
                "executed_at": executed_at,
                "bin": bin_name,
                "label": label,
                "input_dir": normalized_input_dir,
                "case_name": result.case_name,
                "score": result.score if result.status == "ok" else None,
                "elapsed": result.elapsed,
                "status": result.status,
                "stdout_path": result.stdout_path,
            }
        )
    return records


def main() -> int:
    args = parse_args()
    ensure_solver_exists(args.bin_name)

    input_dir = Path(args.input_dir).resolve()
    if not input_dir.is_dir():
        raise SystemExit(f"error: input directory not found: {args.input_dir}")

    input_files = list_input_files(input_dir)
    ensure_unique_basenames(input_files)

    normalized_input_dir = normalize_dir(input_dir)
    is_tools_in = input_dir == DEFAULT_INPUT_DIR.resolve()

    if not args.dry_run:
        ensure_csv_header(SUMMARY_CSV, SUMMARY_HEADER)
        if is_tools_in:
            ensure_csv_header(DETAIL_CSV, compute_detail_header())

    output_dir = ROOT_DIR / "results" / "out" / args.bin_name
    clean_output_dir(output_dir)

    if args.verbose:
        eprint(
            f"eval: bin={args.bin_name} input_dir={normalized_input_dir} "
            f"parallel={args.jobs} timeout_ms={args.timeout_ms} output={output_dir}"
        )

    build_binary(SOLVER_MANIFEST, args.bin_name)

    solver_bin = SOLVER_BIN_DIR / args.bin_name
    if not solver_bin.is_file():
        raise SystemExit(f"error: solver binary not found: {solver_bin}")

    executed_dt = datetime.now().astimezone()
    executed_at = executed_dt.isoformat(timespec="seconds")
    run_id = make_run_id(executed_dt, args.bin_name)

    warm_up_case(
        case_path=input_files[0],
        solver_bin=solver_bin,
        output_dir=output_dir,
        verbose=args.verbose,
        timeout_ms=args.timeout_ms,
    )

    results = evaluate_cases(
        input_files=input_files,
        solver_bin=solver_bin,
        output_dir=output_dir,
        jobs=args.jobs,
        verbose=args.verbose,
        timeout_ms=args.timeout_ms,
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
        append_jsonl(
            RECORDS_JSONL,
            make_records(
                results=results,
                run_id=run_id,
                executed_at=executed_at,
                bin_name=args.bin_name,
                label=args.label,
                normalized_input_dir=normalized_input_dir,
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


if __name__ == "__main__":
    raise SystemExit(main())
