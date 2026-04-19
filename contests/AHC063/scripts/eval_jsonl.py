#!/usr/bin/env python3
from __future__ import annotations
import argparse
import json
import re
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOLVER_MANIFEST = ROOT / "Cargo.toml"
TOOLS_MANIFEST = ROOT / "tools" / "Cargo.toml"
SOLVER_BIN_DIR = ROOT / "target" / "release"
SCORE_BIN = ROOT / "tools" / "target" / "release" / "score"
DEFAULT_BINS = [
    "v012_simple_beam",
    "v014_lane_split_beam",
    "v139_refactor_v137",
    "v149_no_logs",
]
DEFAULT_INPUT_DIRS = [
    ROOT / "tools" / "in",
    ROOT / "tools" / "in_generated",
]
SCORE_RE = re.compile(rb"Score\s*=\s*(-?\d+)")


@dataclass(frozen=True)
class CaseSpec:
    input_path: Path
    dataset: str
    case_name: str
    case_id: str
    relative_path: str
    N: int
    M: int
    C: int


@dataclass
class Summary:
    total: int = 0
    ok: int = 0
    fail: int = 0
    tle: int = 0
    elapsed_total_ms: int = 0
    max_elapsed_ms: int = 0
    score_total: int = 0
    score_count: int = 0

    def update(self, record: dict) -> None:
        elapsed_ms = int(record["elapsed_ms"])
        self.total += 1
        if record["status"] == "ok":
            self.ok += 1
        else:
            self.fail += 1
        if record["tle_2sec"]:
            self.tle += 1
        self.elapsed_total_ms += elapsed_ms
        self.max_elapsed_ms = max(self.max_elapsed_ms, elapsed_ms)
        if record["score"] is not None:
            self.score_total += int(record["score"])
            self.score_count += 1


def eprint(*args) -> None:
    print(*args, file=sys.stderr, flush=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate solver bins serially and append per-case JSONL records."
    )
    parser.add_argument(
        "--bin",
        action="append",
        dest="bins",
        help="solver bin name; repeatable. Defaults to the four configured bins.",
    )
    parser.add_argument(
        "--input-dir",
        action="append",
        dest="input_dirs",
        help="input directory; repeatable. Defaults to tools/in then tools/in_generated.",
    )
    parser.add_argument(
        "--out",
        help="output JSONL path. Defaults to results/analysis/serial_eval_<timestamp>.jsonl",
    )
    parser.add_argument(
        "--timeout-sec",
        type=float,
        default=10.0,
        help="hard timeout per solver execution in seconds (default: 10)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        help="limit the number of cases after concatenating input dirs; useful for smoke tests",
    )
    args = parser.parse_args()
    if args.timeout_sec <= 0:
        parser.error("--timeout-sec must be positive")
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def load_solver_bins() -> set[str]:
    cp = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
            str(SOLVER_MANIFEST),
        ],
        check=True,
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    metadata = json.loads(cp.stdout)
    manifest_abs = SOLVER_MANIFEST.resolve()
    for package in metadata.get("packages", []):
        if Path(package["manifest_path"]).resolve() != manifest_abs:
            continue
        return {
            target["name"]
            for target in package.get("targets", [])
            if "bin" in target.get("kind", [])
        }
    raise SystemExit(f"error: package not found for manifest: {SOLVER_MANIFEST}")


def resolve_bins(raw_bins: list[str] | None) -> list[str]:
    bins = raw_bins or list(DEFAULT_BINS)
    known_bins = load_solver_bins()
    missing = [bin_name for bin_name in bins if bin_name not in known_bins]
    if missing:
        raise SystemExit(f"error: unknown solver bins: {', '.join(missing)}")
    return bins


def resolve_input_dirs(raw_input_dirs: list[str] | None) -> list[Path]:
    resolved = []
    seen_dataset_names: set[str] = set()
    for raw_path in raw_input_dirs or [str(path) for path in DEFAULT_INPUT_DIRS]:
        input_dir = Path(raw_path).expanduser().resolve()
        if not input_dir.is_dir():
            raise SystemExit(f"error: input directory not found: {raw_path}")
        dataset = input_dir.name
        if dataset in seen_dataset_names:
            raise SystemExit(
                f"error: duplicate dataset name '{dataset}' from input dirs"
            )
        seen_dataset_names.add(dataset)
        resolved.append(input_dir)
    return resolved


def default_output_path() -> Path:
    timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return ROOT / "results" / "analysis" / f"serial_eval_{timestamp}.jsonl"


def derive_failure_root(output_path: Path) -> Path:
    return output_path.parent / output_path.stem


def parse_case_header(input_path: Path) -> tuple[int, int, int]:
    with input_path.open() as fh:
        first_line = fh.readline().strip()
    parts = first_line.split()
    if len(parts) != 3:
        raise SystemExit(f"error: malformed first line in {input_path}")
    try:
        N, M, C = (int(part) for part in parts)
    except ValueError as exc:
        raise SystemExit(f"error: failed to parse N M C in {input_path}") from exc
    return N, M, C


def collect_cases(input_dirs: list[Path], limit: int | None) -> list[CaseSpec]:
    cases: list[CaseSpec] = []
    for input_dir in input_dirs:
        files = sorted(path for path in input_dir.rglob("*") if path.is_file())
        if not files:
            raise SystemExit(f"error: input directory is empty: {input_dir}")
        dataset = input_dir.name
        for input_path in files:
            relative_path = input_path.relative_to(input_dir).as_posix()
            N, M, C = parse_case_header(input_path)
            cases.append(
                CaseSpec(
                    input_path=input_path,
                    dataset=dataset,
                    case_name=input_path.name,
                    case_id=f"{dataset}/{relative_path}",
                    relative_path=relative_path,
                    N=N,
                    M=M,
                    C=C,
                )
            )
    if limit is not None:
        cases = cases[:limit]
    if not cases:
        raise SystemExit("error: no cases selected")
    return cases


def build_solver(bin_name: str) -> None:
    eprint(f"build solver: {bin_name}")
    subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--quiet",
            "--manifest-path",
            str(SOLVER_MANIFEST),
            "--bin",
            bin_name,
        ],
        check=True,
        cwd=ROOT,
    )


def build_score() -> None:
    eprint("build scorer: score")
    subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--quiet",
            "--manifest-path",
            str(TOOLS_MANIFEST),
            "--bin",
            "score",
        ],
        check=True,
        cwd=ROOT,
    )


def parse_score(score_stdout: bytes) -> int | None:
    match = SCORE_RE.search(score_stdout)
    if match is None:
        return None
    return int(match.group(1))


def combine_failure_stderr(
    solver_stderr: bytes,
    score_stdout: bytes = b"",
    score_stderr: bytes = b"",
) -> bytes:
    sections: list[bytes] = []
    if solver_stderr:
        sections.append(b"== solver stderr ==\n" + solver_stderr.rstrip(b"\n") + b"\n")
    if score_stdout:
        sections.append(b"== score stdout ==\n" + score_stdout.rstrip(b"\n") + b"\n")
    if score_stderr:
        sections.append(b"== score stderr ==\n" + score_stderr.rstrip(b"\n") + b"\n")
    return b"".join(sections)


def save_failure_artifacts(
    failure_root: Path,
    bin_name: str,
    case: CaseSpec,
    stdout_bytes: bytes,
    stderr_bytes: bytes,
) -> None:
    case_dir = failure_root / bin_name / case.dataset / case.relative_path
    case_dir.mkdir(parents=True, exist_ok=True)
    (case_dir / "stdout.txt").write_bytes(stdout_bytes)
    (case_dir / "stderr.txt").write_bytes(stderr_bytes)


def evaluate_case(
    bin_name: str,
    case: CaseSpec,
    timeout_sec: float,
    scratch_output_path: Path,
    failure_root: Path,
) -> dict:
    input_bytes = case.input_path.read_bytes()
    solver_cmd = [str(SOLVER_BIN_DIR / bin_name)]
    run_start_ns = time.perf_counter_ns()

    try:
        solver_cp = subprocess.run(
            solver_cmd,
            input=input_bytes,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=ROOT,
            timeout=timeout_sec,
            check=False,
        )
        elapsed_ms = (time.perf_counter_ns() - run_start_ns) // 1_000_000
    except subprocess.TimeoutExpired as exc:
        elapsed_ms = (time.perf_counter_ns() - run_start_ns) // 1_000_000
        stdout_bytes = exc.stdout or b""
        stderr_bytes = exc.stderr or b""
        save_failure_artifacts(
            failure_root=failure_root,
            bin_name=bin_name,
            case=case,
            stdout_bytes=stdout_bytes,
            stderr_bytes=combine_failure_stderr(stderr_bytes),
        )
        return {
            "run_id": "",
            "bin": bin_name,
            "dataset": case.dataset,
            "case_name": case.case_name,
            "case_id": case.case_id,
            "N": case.N,
            "M": case.M,
            "C": case.C,
            "elapsed_ms": int(elapsed_ms),
            "tle_2sec": True,
            "score": None,
            "status": "timeout",
        }

    stdout_bytes = solver_cp.stdout
    stderr_bytes = solver_cp.stderr
    tle_2sec = elapsed_ms >= 2000

    if solver_cp.returncode != 0:
        save_failure_artifacts(
            failure_root=failure_root,
            bin_name=bin_name,
            case=case,
            stdout_bytes=stdout_bytes,
            stderr_bytes=combine_failure_stderr(stderr_bytes),
        )
        return {
            "run_id": "",
            "bin": bin_name,
            "dataset": case.dataset,
            "case_name": case.case_name,
            "case_id": case.case_id,
            "N": case.N,
            "M": case.M,
            "C": case.C,
            "elapsed_ms": int(elapsed_ms),
            "tle_2sec": tle_2sec,
            "score": None,
            "status": "run_fail",
        }

    scratch_output_path.write_bytes(stdout_bytes)
    score_cp = subprocess.run(
        [str(SCORE_BIN), str(case.input_path), str(scratch_output_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=ROOT,
        check=False,
    )
    score_stdout = score_cp.stdout
    score_stderr = score_cp.stderr

    if score_cp.returncode != 0:
        save_failure_artifacts(
            failure_root=failure_root,
            bin_name=bin_name,
            case=case,
            stdout_bytes=stdout_bytes,
            stderr_bytes=combine_failure_stderr(stderr_bytes, score_stdout, score_stderr),
        )
        return {
            "run_id": "",
            "bin": bin_name,
            "dataset": case.dataset,
            "case_name": case.case_name,
            "case_id": case.case_id,
            "N": case.N,
            "M": case.M,
            "C": case.C,
            "elapsed_ms": int(elapsed_ms),
            "tle_2sec": tle_2sec,
            "score": None,
            "status": "score_fail",
        }

    score = parse_score(score_stdout)
    if score is None:
        save_failure_artifacts(
            failure_root=failure_root,
            bin_name=bin_name,
            case=case,
            stdout_bytes=stdout_bytes,
            stderr_bytes=combine_failure_stderr(stderr_bytes, score_stdout, score_stderr),
        )
        return {
            "run_id": "",
            "bin": bin_name,
            "dataset": case.dataset,
            "case_name": case.case_name,
            "case_id": case.case_id,
            "N": case.N,
            "M": case.M,
            "C": case.C,
            "elapsed_ms": int(elapsed_ms),
            "tle_2sec": tle_2sec,
            "score": None,
            "status": "score_parse_fail",
        }

    if score <= 0:
        save_failure_artifacts(
            failure_root=failure_root,
            bin_name=bin_name,
            case=case,
            stdout_bytes=stdout_bytes,
            stderr_bytes=combine_failure_stderr(stderr_bytes, score_stdout, score_stderr),
        )
        return {
            "run_id": "",
            "bin": bin_name,
            "dataset": case.dataset,
            "case_name": case.case_name,
            "case_id": case.case_id,
            "N": case.N,
            "M": case.M,
            "C": case.C,
            "elapsed_ms": int(elapsed_ms),
            "tle_2sec": tle_2sec,
            "score": None,
            "status": "score_fail",
        }

    return {
        "run_id": "",
        "bin": bin_name,
        "dataset": case.dataset,
        "case_name": case.case_name,
        "case_id": case.case_id,
        "N": case.N,
        "M": case.M,
        "C": case.C,
        "elapsed_ms": int(elapsed_ms),
        "tle_2sec": tle_2sec,
        "score": int(score),
        "status": "ok",
    }


def print_summary(summary_by_bin: dict[str, Summary]) -> None:
    eprint("")
    eprint("summary by bin")
    header = (
        f"{'bin':<24} {'ok':>5} {'fail':>5} {'tle':>5} "
        f"{'avg_ms':>8} {'max_ms':>8} {'avg_score':>12} {'sum_score':>12}"
    )
    eprint(header)
    eprint("-" * len(header))
    for bin_name, stats in summary_by_bin.items():
        avg_elapsed = stats.elapsed_total_ms / stats.total if stats.total else 0.0
        avg_score = stats.score_total / stats.score_count if stats.score_count else None
        avg_score_text = "-" if avg_score is None else f"{avg_score:.1f}"
        eprint(
            f"{bin_name:<24} {stats.ok:>5} {stats.fail:>5} {stats.tle:>5} "
            f"{avg_elapsed:>8.1f} {stats.max_elapsed_ms:>8} "
            f"{avg_score_text:>12} {stats.score_total:>12}"
        )


def main() -> int:
    args = parse_args()
    bins = resolve_bins(args.bins)
    input_dirs = resolve_input_dirs(args.input_dirs)
    cases = collect_cases(input_dirs, args.limit)

    output_path = Path(args.out).expanduser() if args.out else default_output_path()
    failure_root = derive_failure_root(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    failure_root.mkdir(parents=True, exist_ok=True)

    eprint(f"output jsonl: {output_path}")
    eprint(f"failure root: {failure_root / 'failures'}")
    eprint(
        "config: bins=%d datasets=%d cases=%d records=%d timeout_sec=%.3f"
        % (len(bins), len(input_dirs), len(cases), len(bins) * len(cases), args.timeout_sec)
    )

    for bin_name in bins:
        build_solver(bin_name)
    build_score()

    run_id = output_path.stem
    summary_by_bin = {bin_name: Summary() for bin_name in bins}
    total_records = len(bins) * len(cases)
    done_records = 0

    with tempfile.TemporaryDirectory(prefix="eval_jsonl_", dir=str(ROOT / "results")) as tmpdir:
        scratch_output_path = Path(tmpdir) / "candidate.txt"
        with output_path.open("w", encoding="utf-8") as out_fh:
            for bin_name in bins:
                for case in cases:
                    done_records += 1
                    record = evaluate_case(
                        bin_name=bin_name,
                        case=case,
                        timeout_sec=args.timeout_sec,
                        scratch_output_path=scratch_output_path,
                        failure_root=failure_root / "failures",
                    )
                    record["run_id"] = run_id
                    out_fh.write(json.dumps(record, ensure_ascii=False) + "\n")
                    out_fh.flush()
                    summary_by_bin[bin_name].update(record)
                    score_text = "-" if record["score"] is None else str(record["score"])
                    eprint(
                        f"[{done_records}/{total_records}] "
                        f"{bin_name} {record['case_id']} "
                        f"status={record['status']} score={score_text} "
                        f"elapsed_ms={record['elapsed_ms']}"
                    )

    print_summary(summary_by_bin)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
