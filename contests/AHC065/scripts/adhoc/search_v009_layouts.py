#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
from typing import Optional


ROOT = Path(__file__).resolve().parents[2]
SOLVER_BIN_NAME = "v009_layout_probe_fast"
SOLVER_BIN = ROOT / "target" / "release" / SOLVER_BIN_NAME
SCORE_BIN = ROOT / "tools" / "target" / "release" / "score"
DEFAULT_INPUT_DIR = ROOT / "tools" / "in"
OUT_CSV = ROOT / "results" / "layout_search" / "v009_layouts.csv"
CASE_CSV = ROOT / "results" / "layout_search" / "v009_layout_cases.csv"
OUTPUT_ROOT = ROOT / "results" / "out" / SOLVER_BIN_NAME
SUMMARY_HEADER = [
    "executed_at",
    "candidate",
    "spec",
    "status",
    "success",
    "failure",
    "total_avg",
    "total_sum",
    "total_min",
    "total_max",
    "avg_elapsed",
    "max_elapsed",
    "input_dir",
    "total_cases",
    "output_dir",
]
CASE_HEADER = [
    "executed_at",
    "candidate",
    "spec",
    "input_dir",
    "case_name",
    "status",
    "score",
    "elapsed",
    "stdout_path",
]


@dataclass(frozen=True)
class Candidate:
    name: str
    spec: str


@dataclass(frozen=True)
class CaseResult:
    case_name: str
    status: str
    score: Optional[int]
    elapsed: int


@dataclass(frozen=True)
class CandidateResult:
    candidate: Candidate
    status: str
    success: int
    failure: int
    total_avg: int
    total_sum: int
    total_min: int
    total_max: int
    avg_elapsed: int
    max_elapsed: int
    output_dir: Path
    cases: list[CaseResult]


def eprint(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def parse_csv_ints(raw: str) -> list[int]:
    values = []
    for token in raw.split(","):
        token = token.strip()
        if token:
            values.append(int(token))
    if not values:
        raise argparse.ArgumentTypeError("empty integer list")
    return values


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Evaluate conveyor layouts with the fast v009 ops optimizer. "
            "The solver is built once, then each candidate is selected through "
            "AHC065_LAYOUT_SPEC."
        )
    )
    parser.add_argument(
        "input_dir",
        nargs="?",
        default=str(DEFAULT_INPUT_DIR),
        help="Input directory to evaluate (default: tools/in).",
    )
    parser.add_argument(
        "--candidates",
        choices=["blocks", "structure", "comb"],
        default="blocks",
        help="Candidate source (default: blocks). Use structure or comb for layout families.",
    )
    parser.add_argument(
        "--h-values",
        type=parse_csv_ints,
        default=parse_csv_ints("2,4,6,8,10,20"),
        help="Comma-separated h values for block-width candidates.",
    )
    parser.add_argument(
        "--v-values",
        type=parse_csv_ints,
        default=parse_csv_ints("2,4,6,8,10,20"),
        help="Comma-separated v values for block-width candidates.",
    )
    parser.add_argument(
        "--vsplit",
        default="exit",
        help="Vertical split mode. Only exit is supported for the current block-only search.",
    )
    parser.add_argument(
        "--spec",
        action="append",
        default=[],
        metavar="NAME:SPEC",
        help="Add a custom block-width candidate, e.g. --spec local:h=4,v=6,vs=exit.",
    )
    parser.add_argument(
        "--only",
        default="",
        help="Comma-separated candidate names to run after candidate generation.",
    )
    parser.add_argument(
        "--exclude",
        default="",
        help="Comma-separated candidate names to remove after candidate generation.",
    )
    parser.add_argument("--limit", type=int, default=0, help="Run only the first N candidates.")
    parser.add_argument("--case-limit", type=int, default=0, help="Run only the first N cases.")
    parser.add_argument("--jobs", type=int, default=1, help="Parallel cases per candidate.")
    parser.add_argument("--solver-timeout", type=float, default=8.0, help="Seconds before killing one solver run.")
    parser.add_argument("--no-build", action="store_true", help="Skip cargo build.")
    parser.add_argument("--no-warmup", action="store_true", help="Skip one discarded warmup run per candidate.")
    parser.add_argument("--keep-output", action="store_true", help="Do not delete existing per-candidate output dirs.")
    parser.add_argument("--dry-run", action="store_true", help="Do not append results/layout_search/v009_layouts.csv.")
    parser.add_argument("--top", type=int, default=8, help="Print top N candidates by total_avg.")
    parser.add_argument("-v", "--verbose", action="store_true", help="Show per-case logs.")
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be >= 1")
    if args.limit < 0 or args.case_limit < 0:
        parser.error("--limit and --case-limit must be >= 0")
    if args.candidates == "blocks" and args.vsplit not in {"exit", "around_exit"}:
        parser.error("--vsplit is fixed to exit for the current block-only search")
    return args


def normalize_dir(path: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return resolved.as_posix()


def round_half_up(numerator: int, denominator: int) -> int:
    return int(
        (Decimal(numerator) / Decimal(denominator)).quantize(
            Decimal("1"), rounding=ROUND_HALF_UP
        )
    )


def safe_name(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.=-]+", "_", name).strip("_") or "candidate"


def list_input_files(input_dir: Path, case_limit: int) -> list[Path]:
    files = sorted(path for path in input_dir.rglob("*") if path.is_file())
    if case_limit > 0:
        files = files[:case_limit]
    if not files:
        raise SystemExit(f"error: input directory is empty: {input_dir}")
    seen: dict[str, Path] = {}
    duplicates = []
    for path in files:
        if path.name in seen:
            duplicates.append(path.name)
        seen[path.name] = path
    if duplicates:
        raise SystemExit(f"error: duplicated input basename(s): {', '.join(sorted(set(duplicates)))}")
    return files


def build_binaries() -> None:
    commands = [
        ["cargo", "build", "--release", "--quiet", "--manifest-path", str(ROOT / "Cargo.toml"), "--bin", SOLVER_BIN_NAME],
        ["cargo", "build", "--release", "--quiet", "--manifest-path", str(ROOT / "tools" / "Cargo.toml"), "--bin", "score"],
    ]
    for command in commands:
        result = subprocess.run(command, cwd=ROOT)
        if result.returncode != 0:
            raise SystemExit(result.returncode)
    if not SOLVER_BIN.is_file():
        raise SystemExit(f"error: solver binary not found: {SOLVER_BIN}")
    if not SCORE_BIN.is_file():
        raise SystemExit(f"error: scorer binary not found: {SCORE_BIN}")


def parse_score(stdout: str) -> Optional[int]:
    tokens = stdout.split()
    if not tokens:
        return None
    try:
        return int(tokens[-1])
    except ValueError:
        return None


def run_case(
    candidate: Candidate,
    case_path: Path,
    output_dir: Path,
    verbose: bool,
    solver_timeout: float,
) -> CaseResult:
    case_name = case_path.name
    output_path = output_dir / case_name
    err_path = output_dir / f"{case_name}.err"
    env = os.environ.copy()
    env["AHC065_LAYOUT_SPEC"] = candidate.spec

    if verbose:
        eprint(f"start: {candidate.name} {case_name}")

    start_ns = time.monotonic_ns()
    try:
        with case_path.open("rb") as fin, output_path.open("wb") as fout, err_path.open("wb") as ferr:
            run_result = subprocess.run(
                [str(SOLVER_BIN)],
                stdin=fin,
                stdout=fout,
                stderr=ferr,
                env=env,
                timeout=solver_timeout,
            )
    except subprocess.TimeoutExpired:
        elapsed = int((time.monotonic_ns() - start_ns) // 1_000_000)
        return CaseResult(case_name, "run_timeout", None, elapsed)
    except OSError:
        elapsed = int((time.monotonic_ns() - start_ns) // 1_000_000)
        return CaseResult(case_name, "run_fail", None, elapsed)

    run_elapsed = int((time.monotonic_ns() - start_ns) // 1_000_000)
    if run_result.returncode != 0:
        return CaseResult(case_name, "run_fail", None, run_elapsed)

    score_start_ns = time.monotonic_ns()
    try:
        with err_path.open("ab") as ferr:
            score_result = subprocess.run(
                [str(SCORE_BIN), str(case_path), str(output_path)],
                stdout=subprocess.PIPE,
                stderr=ferr,
                text=True,
            )
    except OSError:
        elapsed = run_elapsed + int((time.monotonic_ns() - score_start_ns) // 1_000_000)
        return CaseResult(case_name, "score_fail", None, elapsed)

    score_elapsed = int((time.monotonic_ns() - score_start_ns) // 1_000_000)
    elapsed = run_elapsed + score_elapsed
    if score_result.returncode != 0:
        return CaseResult(case_name, "score_fail", None, elapsed)

    score = parse_score(score_result.stdout)
    if score is None:
        return CaseResult(case_name, "score_parse_fail", None, elapsed)

    if verbose:
        eprint(f"done: {candidate.name} {case_name} score={score} elapsed={elapsed}ms")
    return CaseResult(case_name, "ok", score, elapsed)


def warmup(candidate: Candidate, case_path: Path, output_dir: Path, solver_timeout: float, verbose: bool) -> None:
    temp_dir = output_dir / ".warmup"
    if temp_dir.exists():
        shutil.rmtree(temp_dir)
    temp_dir.mkdir(parents=True, exist_ok=True)
    result = run_case(candidate, case_path, temp_dir, False, solver_timeout)
    if verbose:
        score = "" if result.score is None else f" score={result.score}"
        eprint(f"warmup: {candidate.name} {result.case_name} status={result.status}{score}")
    shutil.rmtree(temp_dir, ignore_errors=True)


def evaluate_candidate(
    candidate: Candidate,
    input_files: list[Path],
    args: argparse.Namespace,
    executed_at: str,
    input_dir: str,
) -> CandidateResult:
    output_dir = OUTPUT_ROOT / safe_name(candidate.name)
    if output_dir.exists() and not args.keep_output:
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    if not args.no_warmup:
        warmup(candidate, input_files[0], output_dir, args.solver_timeout, args.verbose)

    if args.jobs == 1:
        results = []
        for path in input_files:
            result = run_case(candidate, path, output_dir, args.verbose, args.solver_timeout)
            results.append(result)
            if not args.dry_run:
                append_case_result(CASE_CSV, executed_at, candidate, input_dir, output_dir, result)
    else:
        by_name: dict[str, CaseResult] = {}
        with ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futures = {
                executor.submit(
                    run_case,
                    candidate,
                    path,
                    output_dir,
                    args.verbose,
                    args.solver_timeout,
                ): path
                for path in input_files
            }
            for future in as_completed(futures):
                result = future.result()
                by_name[result.case_name] = result
                if not args.dry_run:
                    append_case_result(CASE_CSV, executed_at, candidate, input_dir, output_dir, result)
        results = [by_name[path.name] for path in input_files]

    ok = [result for result in results if result.status == "ok" and result.score is not None]
    success = len(ok)
    failure = len(results) - success
    total_sum = sum(result.score for result in ok if result.score is not None)
    total_avg = round_half_up(total_sum, success) if success else 0
    total_min = min((result.score for result in ok if result.score is not None), default=0)
    total_max = max((result.score for result in ok if result.score is not None), default=0)
    avg_elapsed = round_half_up(sum(result.elapsed for result in ok), success) if success else 0
    max_elapsed = max((result.elapsed for result in ok), default=0)
    status = "ok" if failure == 0 else "fail"

    eprint(
        "layout: "
        f"candidate={candidate.name} status={status} success={success} failure={failure} "
        f"total_avg={total_avg} total_sum={total_sum} avg_elapsed={avg_elapsed} "
        f"max_elapsed={max_elapsed} spec={candidate.spec}"
    )
    return CandidateResult(
        candidate=candidate,
        status=status,
        success=success,
        failure=failure,
        total_avg=total_avg,
        total_sum=total_sum,
        total_min=total_min,
        total_max=total_max,
        avg_elapsed=avg_elapsed,
        max_elapsed=max_elapsed,
        output_dir=output_dir,
        cases=results,
    )


def ensure_csv_header(path: Path, header: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.stat().st_size > 0:
        with path.open("r", encoding="utf-8", newline="") as handle:
            actual = handle.readline().rstrip("\r\n")
        expected = ",".join(header)
        if actual != expected:
            raise SystemExit(f"error: CSV header mismatch: {path}")
        return
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(",".join(header) + "\n")


def append_result(path: Path, executed_at: str, result: CandidateResult, input_dir: str, total_cases: int) -> None:
    ensure_csv_header(path, SUMMARY_HEADER)
    with path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                executed_at,
                result.candidate.name,
                result.candidate.spec,
                result.status,
                result.success,
                result.failure,
                result.total_avg,
                result.total_sum,
                result.total_min,
                result.total_max,
                result.avg_elapsed,
                result.max_elapsed,
                input_dir,
                total_cases,
                result.output_dir.relative_to(ROOT).as_posix(),
            ]
        )


def append_case_result(
    path: Path,
    executed_at: str,
    candidate: Candidate,
    input_dir: str,
    output_dir: Path,
    case: CaseResult,
) -> None:
    ensure_csv_header(path, CASE_HEADER)
    with path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        stdout_path = (output_dir / case.case_name).relative_to(ROOT).as_posix()
        writer.writerow(
            [
                executed_at,
                candidate.name,
                candidate.spec,
                input_dir,
                case.case_name,
                case.status,
                "" if case.score is None else case.score,
                case.elapsed,
                stdout_path,
            ]
        )


def block_candidates(args: argparse.Namespace) -> list[Candidate]:
    candidates = []
    for h in args.h_values:
        for v in args.v_values:
            candidates.append(Candidate(f"h{h}_v{v}_exit", f"h={h},v={v},vs=exit"))
    return candidates


def structure_candidate_name(vs: str, order: str, ro: int, co: int, hr: int, vr: int) -> str:
    parts = ["h4", "v4", vs]
    if ro != 0:
        parts.append(f"ro{ro}")
    if co != 0:
        parts.append(f"co{co}")
    if order != "hv":
        parts.append(order)
    if hr:
        parts.append("hr")
    if vr:
        parts.append("vr")
    return "_".join(parts)


def structure_candidates() -> list[Candidate]:
    candidates = []
    for vs in ["exit", "grid"]:
        for order in ["hv", "vh"]:
            for ro in [0, 2]:
                for co in [0, 2]:
                    for hr in [0, 1]:
                        for vr in [0, 1]:
                            name = structure_candidate_name(vs, order, ro, co, hr, vr)
                            fields = [
                                "h=4",
                                "v=4",
                                f"vs={vs}",
                                f"order={order}",
                                f"ro={ro}",
                                f"co={co}",
                                f"hr={hr}",
                                f"vr={vr}",
                            ]
                            candidates.append(Candidate(name, ",".join(fields)))
    return candidates


def comb_candidates() -> list[Candidate]:
    candidates = []
    for vc in [4, 6]:
        for hc in [4, 6]:
            for vside in ["left", "right", "alt"]:
                for hside in ["top", "bottom", "alt"]:
                    name = f"comb_V{vc}_H{hc}_{vside}_{hside}"
                    spec = (
                        f"family=comb,vc={vc},hc={hc},val=18,hal=18,"
                        f"vside={vside},hside={hside}"
                    )
                    candidates.append(Candidate(name, spec))
    return candidates


def parse_custom_spec(raw: str) -> Candidate:
    if ":" in raw:
        name, spec = raw.split(":", 1)
    elif "=" in raw:
        name, spec = raw.split("=", 1)
    else:
        raise SystemExit(f"error: custom spec must be NAME:SPEC: {raw}")
    name = name.strip()
    spec = spec.strip()
    if not name or not spec:
        raise SystemExit(f"error: custom spec must be NAME:SPEC: {raw}")
    return Candidate(name, spec)


def select_candidates(args: argparse.Namespace) -> list[Candidate]:
    if args.candidates == "blocks":
        candidates = block_candidates(args)
    elif args.candidates == "structure":
        candidates = structure_candidates()
    else:
        candidates = comb_candidates()
    candidates.extend(parse_custom_spec(raw) for raw in args.spec)

    seen = set()
    unique = []
    for candidate in candidates:
        if candidate.name in seen:
            raise SystemExit(f"error: duplicated candidate name: {candidate.name}")
        seen.add(candidate.name)
        unique.append(candidate)
    candidates = unique

    if args.only:
        wanted = {name.strip() for name in args.only.split(",") if name.strip()}
        candidates = [candidate for candidate in candidates if candidate.name in wanted]
        missing = wanted.difference(candidate.name for candidate in candidates)
        if missing:
            raise SystemExit(f"error: unknown candidate(s): {', '.join(sorted(missing))}")

    if args.exclude:
        excluded = {name.strip() for name in args.exclude.split(",") if name.strip()}
        known = {candidate.name for candidate in candidates}
        missing = excluded.difference(known)
        if missing:
            raise SystemExit(f"error: unknown excluded candidate(s): {', '.join(sorted(missing))}")
        candidates = [candidate for candidate in candidates if candidate.name not in excluded]

    if args.limit > 0:
        candidates = candidates[: args.limit]
    if not candidates:
        raise SystemExit("error: no candidates selected")
    return candidates


def print_ranking(results: list[CandidateResult], top: int) -> None:
    if top <= 0:
        return
    ranked = sorted(results, key=lambda r: (r.failure, -r.total_avg, r.avg_elapsed, r.candidate.name))
    print("\nrank,candidate,total_avg,total_sum,failure,avg_elapsed,max_elapsed,spec")
    for rank, result in enumerate(ranked[:top], start=1):
        print(
            f"{rank},{result.candidate.name},{result.total_avg},{result.total_sum},"
            f"{result.failure},{result.avg_elapsed},{result.max_elapsed},{result.candidate.spec}"
        )


def main() -> int:
    args = parse_args()
    input_dir = Path(args.input_dir).resolve()
    if not input_dir.is_dir():
        raise SystemExit(f"error: input directory not found: {input_dir}")

    input_files = list_input_files(input_dir, args.case_limit)
    candidates = select_candidates(args)
    normalized_input_dir = normalize_dir(input_dir)

    if not args.no_build:
        build_binaries()
    elif not SOLVER_BIN.is_file() or not SCORE_BIN.is_file():
        raise SystemExit("error: --no-build was set but required binaries are missing")

    eprint(
        f"layout_search: candidates={len(candidates)} cases={len(input_files)} "
        f"input_dir={normalized_input_dir} jobs={args.jobs}"
    )

    executed_at = datetime.now().astimezone().isoformat(timespec="seconds")
    results = []
    for candidate in candidates:
        result = evaluate_candidate(candidate, input_files, args, executed_at, normalized_input_dir)
        results.append(result)
        if not args.dry_run:
            append_result(OUT_CSV, executed_at, result, normalized_input_dir, len(input_files))

    print_ranking(results, args.top)
    return 0 if all(result.failure == 0 for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
