#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import os
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
TOOLS_MANIFEST = ROOT_DIR / "tools" / "Cargo.toml"
SOLVER_BIN_DIR = ROOT_DIR / "target" / "release"
TOOLS_BIN_DIR = ROOT_DIR / "tools" / "target" / "release"
DEFAULT_INPUT_DIR = ROOT_DIR / "tools" / "in"
SUMMARY_CSV = ROOT_DIR / "results" / "score_summary.csv"
DETAIL_CSV = ROOT_DIR / "results" / "score_detail.csv"
RECORDS_JSONL = ROOT_DIR / "results" / "eval_records.jsonl"

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
            "Build solver and scorer once, warm up with one solver -> score run, "
            "then evaluate solver -> score per case."
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
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("jobs must be >= 1")
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


def ensure_tools_ready() -> None:
    if not TOOLS_MANIFEST.is_file():
        raise SystemExit(f"error: tools manifest not found: {TOOLS_MANIFEST}")


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


def parse_score(stdout: str) -> Optional[int]:
    tokens = stdout.split()
    if not tokens:
        return None
    try:
        return int(tokens[-1])
    except ValueError:
        return None


def run_case(
    case_path: Path,
    solver_bin: Path,
    score_bin: Path,
    output_dir: Path,
    verbose: bool,
) -> CaseResult:
    case_name = case_path.name
    output_path = output_dir / case_name
    err_path = output_dir / f"{case_name}.err"
    stdout_path = output_path.relative_to(ROOT_DIR).as_posix()

    if verbose:
        eprint(f"start: {case_name}")

    run_start_ns = time.monotonic_ns()
    try:
        with case_path.open("rb") as fin, output_path.open("wb") as fout, err_path.open(
            "wb"
        ) as ferr:
            run_result = subprocess.run(
                [str(solver_bin)],
                stdin=fin,
                stdout=fout,
                stderr=ferr,
            )
    except OSError:
        run_elapsed = (time.monotonic_ns() - run_start_ns) // 1_000_000
        return CaseResult(
            case_name=case_name,
            status="run_fail",
            score=None,
            elapsed=int(run_elapsed),
            stdout_path=stdout_path,
        )
    run_elapsed = (time.monotonic_ns() - run_start_ns) // 1_000_000
    if run_result.returncode != 0:
        if verbose:
            eprint(f"fail(run): {case_name}")
        return CaseResult(
            case_name=case_name,
            status="run_fail",
            score=None,
            elapsed=int(run_elapsed),
            stdout_path=stdout_path,
        )

    score_start_ns = time.monotonic_ns()
    try:
        with err_path.open("ab") as ferr:
            score_result = subprocess.run(
                [str(score_bin), str(case_path), str(output_path)],
                stdout=subprocess.PIPE,
                stderr=ferr,
                text=True,
            )
    except OSError:
        score_elapsed = (time.monotonic_ns() - score_start_ns) // 1_000_000
        return CaseResult(
            case_name=case_name,
            status="score_fail",
            score=None,
            elapsed=int(run_elapsed + score_elapsed),
            stdout_path=stdout_path,
        )
    score_elapsed = (time.monotonic_ns() - score_start_ns) // 1_000_000
    total_elapsed = int(run_elapsed + score_elapsed)
    if score_result.returncode != 0:
        if verbose:
            eprint(f"fail(score): {case_name}")
        return CaseResult(
            case_name=case_name,
            status="score_fail",
            score=None,
            elapsed=total_elapsed,
            stdout_path=stdout_path,
        )

    score = parse_score(score_result.stdout)
    if score is None:
        if verbose:
            eprint(f"fail(parse): {case_name}")
        return CaseResult(
            case_name=case_name,
            status="score_parse_fail",
            score=None,
            elapsed=total_elapsed,
            stdout_path=stdout_path,
        )

    if verbose:
        eprint(
            f"done: {case_name} score={score} elapsed={total_elapsed}ms output={stdout_path}"
        )
    return CaseResult(
        case_name=case_name,
        status="ok",
        score=score,
        elapsed=total_elapsed,
        stdout_path=stdout_path,
    )


def evaluate_cases(
    input_files: list[Path],
    solver_bin: Path,
    score_bin: Path,
    output_dir: Path,
    jobs: int,
    verbose: bool,
) -> list[CaseResult]:
    if jobs == 1:
        return [run_case(path, solver_bin, score_bin, output_dir, verbose) for path in input_files]

    results_by_name: dict[str, CaseResult] = {}
    with ThreadPoolExecutor(max_workers=jobs) as executor:
        future_map = {
            executor.submit(run_case, path, solver_bin, score_bin, output_dir, verbose): path
            for path in input_files
        }
        for future in as_completed(future_map):
            result = future.result()
            results_by_name[result.case_name] = result
    return [results_by_name[path.name] for path in input_files]


def warm_up_case(
    case_path: Path,
    solver_bin: Path,
    score_bin: Path,
    output_dir: Path,
    verbose: bool,
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
    ensure_tools_ready()

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
            f"parallel={args.jobs} output={output_dir}"
        )

    build_binary(SOLVER_MANIFEST, args.bin_name)
    build_binary(TOOLS_MANIFEST, "score")

    solver_bin = SOLVER_BIN_DIR / args.bin_name
    score_bin = TOOLS_BIN_DIR / "score"
    if not solver_bin.is_file():
        raise SystemExit(f"error: solver binary not found: {solver_bin}")
    if not score_bin.is_file():
        raise SystemExit(f"error: score binary not found: {score_bin}")

    executed_dt = datetime.now().astimezone()
    executed_at = executed_dt.isoformat(timespec="seconds")
    run_id = make_run_id(executed_dt, args.bin_name)

    warm_up_case(
        case_path=input_files[0],
        solver_bin=solver_bin,
        score_bin=score_bin,
        output_dir=output_dir,
        verbose=args.verbose,
    )

    results = evaluate_cases(
        input_files=input_files,
        solver_bin=solver_bin,
        score_bin=score_bin,
        output_dir=output_dir,
        jobs=args.jobs,
        verbose=args.verbose,
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
