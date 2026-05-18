#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOLVER = ROOT / "src" / "bin" / "v009_pro_conveyor_beam.rs"
EVAL = ROOT / "scripts" / "eval.py"
OUT_CSV = ROOT / "results" / "param_search" / "v009_params.csv"
BIN_NAME = "v009_pro_conveyor_beam"


DEFAULT_CANDIDATES: list[tuple[str, dict[str, str]]] = [
    ("w160_k80_d8_l12", {"BEAM_WIDTH": "160", "K_LOOK": "80", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.2"}),
    ("w240_k80_d8_l12", {"BEAM_WIDTH": "240", "K_LOOK": "80", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.2"}),
    ("w320_k80_d8_l12", {"BEAM_WIDTH": "320", "K_LOOK": "80", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.2"}),
    ("w240_k100_d8_l12", {"BEAM_WIDTH": "240", "K_LOOK": "100", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.2"}),
    ("w240_k80_d10_l12", {"BEAM_WIDTH": "240", "K_LOOK": "80", "EXTRA_DEPTH": "10", "LEN_PENALTY": "1.2"}),
    ("w240_k80_d8_l10", {"BEAM_WIDTH": "240", "K_LOOK": "80", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.0"}),
    ("w240_k80_d8_l14", {"BEAM_WIDTH": "240", "K_LOOK": "80", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.4"}),
    ("w320_k100_d8_l12", {"BEAM_WIDTH": "320", "K_LOOK": "100", "EXTRA_DEPTH": "8", "LEN_PENALTY": "1.2"}),
    ("w320_k80_d10_l12", {"BEAM_WIDTH": "320", "K_LOOK": "80", "EXTRA_DEPTH": "10", "LEN_PENALTY": "1.2"}),
]


SUMMARY_RE = re.compile(
    r"success=(?P<success>\d+) failure=(?P<failure>\d+) "
    r"total_avg=(?P<total_avg>\d+) avg_elapsed=(?P<avg_elapsed>\d+) "
    r"max_elapsed=(?P<max_elapsed>\d+) total_sum=(?P<total_sum>\d+) "
    r"total_min=(?P<total_min>\d+) total_max=(?P<total_max>\d+)"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Temporarily patch v009_pro_conveyor_beam.rs constants and run "
            "./scripts/eval.py v009_pro_conveyor_beam -j 1 for each candidate."
        )
    )
    parser.add_argument(
        "input_dir",
        nargs="?",
        default=None,
        help="Optional eval input directory. Omit to use tools/in.",
    )
    parser.add_argument(
        "--only",
        default="",
        help="Comma-separated candidate names to run.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help="Run only the first N selected candidates.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Pass --dry-run to eval.py so score CSV/JSONL are not updated.",
    )
    return parser.parse_args()


def patch_constants(src: str, params: dict[str, str]) -> str:
    patched = src
    for key, value in params.items():
        pattern = re.compile(rf"^(const {re.escape(key)}: [^=]+ = )[^;]+;", re.MULTILINE)
        patched, count = pattern.subn(rf"\g<1>{value};", patched)
        if count != 1:
            raise RuntimeError(f"failed to patch const {key}: replacements={count}")
    return patched


def ensure_csv_header(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.stat().st_size > 0:
        return
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "executed_at",
                "candidate",
                "params",
                "status",
                "success",
                "failure",
                "total_avg",
                "total_sum",
                "total_min",
                "total_max",
                "avg_elapsed",
                "max_elapsed",
            ]
        )


def append_result(path: Path, row: dict[str, str]) -> None:
    ensure_csv_header(path)
    with path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                row.get("executed_at", ""),
                row.get("candidate", ""),
                row.get("params", ""),
                row.get("status", ""),
                row.get("success", ""),
                row.get("failure", ""),
                row.get("total_avg", ""),
                row.get("total_sum", ""),
                row.get("total_min", ""),
                row.get("total_max", ""),
                row.get("avg_elapsed", ""),
                row.get("max_elapsed", ""),
            ]
        )


def run_eval(candidate: str, params: dict[str, str], args: argparse.Namespace) -> dict[str, str]:
    label = "param:" + candidate
    cmd = [sys.executable, str(EVAL), BIN_NAME]
    if args.input_dir is not None:
        cmd.append(args.input_dir)
    cmd.extend(["-j", "1", "--label", label])
    if args.dry_run:
        cmd.append("--dry-run")

    print(f"==> {candidate} {format_params(params)}", flush=True)
    result = subprocess.run(cmd, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)

    row = {
        "executed_at": datetime.now().astimezone().isoformat(timespec="seconds"),
        "candidate": candidate,
        "params": format_params(params),
        "status": "ok" if result.returncode == 0 else f"exit_{result.returncode}",
    }
    match = SUMMARY_RE.search(result.stdout + "\n" + result.stderr)
    if match:
        row.update(match.groupdict())
    return row


def format_params(params: dict[str, str]) -> str:
    return ";".join(f"{key}={value}" for key, value in sorted(params.items()))


def select_candidates(args: argparse.Namespace) -> list[tuple[str, dict[str, str]]]:
    candidates = DEFAULT_CANDIDATES
    if args.only:
        wanted = {name.strip() for name in args.only.split(",") if name.strip()}
        candidates = [(name, params) for name, params in candidates if name in wanted]
        missing = wanted.difference(name for name, _ in candidates)
        if missing:
            raise SystemExit(f"unknown candidate(s): {', '.join(sorted(missing))}")
    if args.limit > 0:
        candidates = candidates[: args.limit]
    return candidates


def main() -> int:
    args = parse_args()
    original = SOLVER.read_text(encoding="utf-8")
    candidates = select_candidates(args)
    if not candidates:
        raise SystemExit("no candidates selected")

    try:
        for candidate, params in candidates:
            SOLVER.write_text(patch_constants(original, params), encoding="utf-8")
            row = run_eval(candidate, params, args)
            append_result(OUT_CSV, row)
            print(f"<== {candidate} status={row['status']} total_avg={row.get('total_avg', '')}", flush=True)
    finally:
        SOLVER.write_text(original, encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
