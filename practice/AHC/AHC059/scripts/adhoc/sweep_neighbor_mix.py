#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INPUT_DIR = ROOT / "tools" / "in"
ADHOC_DIR = ROOT / "results" / "adhoc"


@dataclass(frozen=True)
class Mix:
    two_opt: int
    swap: int
    relocate: int

    def label(self) -> str:
        return f"t{self.two_opt:02d}_s{self.swap:02d}_r{self.relocate:02d}"


def eprint(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Sweep v003 neighbor ratios by using local-only environment variables."
    )
    parser.add_argument("--bin", default="v003_stack_total_dp")
    parser.add_argument("--input-dir", default=str(DEFAULT_INPUT_DIR))
    parser.add_argument("--cases", type=int, default=24)
    parser.add_argument("--jobs", type=int, default=0, help="0 keeps eval.py default")
    parser.add_argument("--top-full", type=int, default=3)
    parser.add_argument("--dry-run", action="store_true", default=True)
    parser.add_argument(
        "--mix",
        action="append",
        help="Explicit mix as two_opt,swap,relocate. Can be passed multiple times.",
    )
    parser.add_argument("--two-min", type=int, default=40)
    parser.add_argument("--two-max", type=int, default=65)
    parser.add_argument("--swap-min", type=int, default=0)
    parser.add_argument("--swap-max", type=int, default=25)
    parser.add_argument("--relocate-min", type=int, default=25)
    parser.add_argument("--relocate-max", type=int, default=45)
    parser.add_argument("--step", type=int, default=5)
    return parser.parse_args()


def parse_mix(value: str) -> Mix:
    parts = [int(part) for part in value.split(",")]
    if len(parts) != 3:
        raise ValueError(f"invalid mix: {value}")
    mix = Mix(*parts)
    if mix.two_opt + mix.swap + mix.relocate != 100:
        raise ValueError(f"mix must sum to 100: {value}")
    return mix


def make_grid(args: argparse.Namespace) -> list[Mix]:
    if args.mix:
        mixes = [parse_mix(value) for value in args.mix]
    else:
        mixes = []
        for two in range(args.two_min, args.two_max + 1, args.step):
            for swap in range(args.swap_min, args.swap_max + 1, args.step):
                relocate = 100 - two - swap
                if args.relocate_min <= relocate <= args.relocate_max:
                    mixes.append(Mix(two, swap, relocate))

    seen: set[Mix] = set()
    unique = []
    for mix in mixes:
        if mix not in seen:
            seen.add(mix)
            unique.append(mix)
    return unique


def list_inputs(input_dir: Path) -> list[Path]:
    files = sorted(path for path in input_dir.rglob("*") if path.is_file())
    if not files:
        raise SystemExit(f"input dir is empty: {input_dir}")
    return files


def choose_cases(files: list[Path], cases: int) -> list[Path]:
    if cases <= 0 or cases >= len(files):
        return files
    if cases == 1:
        return [files[0]]
    selected = []
    last = len(files) - 1
    for i in range(cases):
        selected.append(files[round(i * last / (cases - 1))])
    return selected


def make_subset_dir(files: list[Path], cases: int) -> Path:
    ADHOC_DIR.mkdir(parents=True, exist_ok=True)
    subset_dir = ADHOC_DIR / f"neighbor_mix_cases_{cases}"
    if subset_dir.exists():
        shutil.rmtree(subset_dir)
    subset_dir.mkdir(parents=True)
    for path in files:
        os.symlink(path.resolve(), subset_dir / path.name)
    return subset_dir


def run_eval(bin_name: str, input_dir: Path, mix: Mix, jobs: int) -> dict[str, str | int]:
    env = os.environ.copy()
    env["AHC_NEIGHBOR_TWO_OPT"] = str(mix.two_opt)
    env["AHC_NEIGHBOR_SWAP"] = str(mix.swap)
    env["AHC_NEIGHBOR_RELOCATE"] = str(mix.relocate)

    command = [
        str(ROOT / "scripts" / "eval.py"),
        bin_name,
        str(input_dir),
        "--dry-run",
        "--label",
        f"neighbor_mix_{mix.label()}",
    ]
    if jobs > 0:
        command.extend(["-j", str(jobs)])

    proc = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    output = proc.stdout + proc.stderr
    if proc.returncode != 0:
        raise RuntimeError(f"eval failed for {mix.label()}\n{output}")

    result: dict[str, str | int] = {
        "mix": mix.label(),
        "two_opt": mix.two_opt,
        "swap": mix.swap,
        "relocate": mix.relocate,
    }
    for key in [
        "success",
        "failure",
        "total_avg",
        "avg_elapsed",
        "max_elapsed",
        "total_sum",
        "total_min",
        "total_max",
        "total_cases",
    ]:
        match = re.search(rf"\b{key}=([0-9]+)", output)
        if match:
            result[key] = int(match.group(1))
    return result


def write_csv(path: Path, rows: list[dict[str, str | int]]) -> None:
    fields = [
        "mix",
        "two_opt",
        "swap",
        "relocate",
        "total_sum",
        "total_avg",
        "total_min",
        "total_max",
        "avg_elapsed",
        "max_elapsed",
        "total_cases",
        "success",
        "failure",
    ]
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field, "") for field in fields})


def main() -> None:
    args = parse_args()
    input_dir = Path(args.input_dir)
    all_inputs = list_inputs(input_dir)
    subset_files = choose_cases(all_inputs, args.cases)
    subset_dir = make_subset_dir(subset_files, len(subset_files))
    mixes = make_grid(args)
    if not mixes:
        raise SystemExit("no mixes to evaluate")

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    screen_csv = ADHOC_DIR / f"neighbor_mix_screen_{timestamp}.csv"
    full_csv = ADHOC_DIR / f"neighbor_mix_full_{timestamp}.csv"

    eprint(f"screen cases={len(subset_files)} mixes={len(mixes)} input={subset_dir}")
    screen_rows = []
    for index, mix in enumerate(mixes, 1):
        eprint(f"screen {index}/{len(mixes)} {mix.label()}")
        row = run_eval(args.bin, subset_dir, mix, args.jobs)
        screen_rows.append(row)
        write_csv(screen_csv, sorted(screen_rows, key=lambda r: int(r["total_sum"]), reverse=True))

    screen_rows.sort(key=lambda r: int(r["total_sum"]), reverse=True)
    eprint(f"screen csv={screen_csv}")
    for row in screen_rows[: min(10, len(screen_rows))]:
        eprint(
            f"screen_top mix={row['mix']} total_sum={row['total_sum']} "
            f"total_avg={row['total_avg']}"
        )

    if args.top_full <= 0:
        return

    top_mixes = [
        Mix(int(row["two_opt"]), int(row["swap"]), int(row["relocate"]))
        for row in screen_rows[: args.top_full]
    ]
    eprint(f"full cases={len(all_inputs)} mixes={len(top_mixes)} input={input_dir}")
    full_rows = []
    for index, mix in enumerate(top_mixes, 1):
        eprint(f"full {index}/{len(top_mixes)} {mix.label()}")
        row = run_eval(args.bin, input_dir, mix, args.jobs)
        full_rows.append(row)
        write_csv(full_csv, sorted(full_rows, key=lambda r: int(r["total_sum"]), reverse=True))

    full_rows.sort(key=lambda r: int(r["total_sum"]), reverse=True)
    eprint(f"full csv={full_csv}")
    for row in full_rows:
        eprint(
            f"full_result mix={row['mix']} total_sum={row['total_sum']} "
            f"total_avg={row['total_avg']}"
        )


if __name__ == "__main__":
    main()
