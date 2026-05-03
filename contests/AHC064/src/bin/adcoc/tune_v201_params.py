#!/usr/bin/env python3
from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import re
import shutil
import subprocess
import sys
import textwrap
import time
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / "src" / "bin" / "v201pro_hybrid.rs"
WORK_DIR = ROOT / "target" / "param_tune_v201"
GEN_DIR = WORK_DIR / "generated"
BIN_DIR = WORK_DIR / "bin"
OUT_DIR = WORK_DIR / "out"
DEFAULT_INPUT_DIR = ROOT / "tools" / "in"
BASELINE_BIN = ROOT / "target" / "release" / "v201pro_hybrid"
SCORER_BIN = ROOT / "tools" / "target" / "release" / "vis"


@dataclasses.dataclass(frozen=True)
class BeamParams:
    width: int
    max_expand: int
    bad_weight: int
    extra_depth: int
    final_keep: int

    def rust(self) -> str:
        return (
            "BeamParams { "
            f"width: {self.width}, max_expand: {self.max_expand}, "
            f"bad_weight: {self.bad_weight}, extra_depth: {self.extra_depth}, "
            f"final_keep: {self.final_keep} "
            "}"
        )


@dataclasses.dataclass(frozen=True)
class Schedule:
    name: str
    params: tuple[BeamParams, ...]
    penalties: tuple[int, ...]


@dataclasses.dataclass(frozen=True)
class CaseResult:
    case_name: str
    score: int | None
    elapsed_ms: int
    status: str


@dataclasses.dataclass(frozen=True)
class ScheduleResult:
    name: str
    cases: int
    failures: int
    avg_score: int
    min_score: int
    max_score: int
    avg_elapsed_ms: int
    max_elapsed_ms: int
    bin_path: Path


BASE = BeamParams(3000, 100, 0, 2, 12)


def eprint(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def unique_params(params: Iterable[BeamParams]) -> tuple[BeamParams, ...]:
    seen: set[BeamParams] = set()
    out: list[BeamParams] = []
    for p in params:
        if p in seen or p == BASE:
            continue
        seen.add(p)
        out.append(p)
    return tuple(out)


def candidate_orders() -> dict[str, tuple[BeamParams, ...]]:
    keep = [
        BeamParams(3000, 100, 0, 2, k)
        for k in [24, 40, 80, 120]
    ]
    width = [
        BeamParams(w, 100, 0, 2, 40)
        for w in [4500, 6000, 8000, 10000]
    ]
    expand = [
        BeamParams(w, e, 0, 2, 40)
        for w in [4500, 6000]
        for e in [140, 180]
    ]
    depth = [
        BeamParams(3000, 100, 0, 3, 40),
        BeamParams(3000, 100, 0, 3, 80),
        BeamParams(4500, 100, 0, 3, 40),
        BeamParams(4500, 140, 0, 3, 40),
        BeamParams(6000, 140, 0, 3, 40),
    ]
    bad = [
        BeamParams(3000, 100, b, 2, 40)
        for b in [1, 2, 4, 8]
    ] + [
        BeamParams(4500, 100, b, 2, 40)
        for b in [1, 2, 4]
    ]
    mixed = [
        BeamParams(3000, 100, 0, 2, 24),
        BeamParams(4500, 100, 0, 2, 40),
        BeamParams(3000, 100, 1, 2, 40),
        BeamParams(3000, 100, 2, 2, 40),
        BeamParams(3000, 100, 0, 2, 40),
        BeamParams(6000, 100, 0, 2, 40),
        BeamParams(4500, 140, 0, 2, 40),
        BeamParams(3000, 100, 0, 3, 40),
        BeamParams(3000, 100, 4, 2, 40),
        BeamParams(3000, 100, 0, 2, 80),
        BeamParams(6000, 140, 0, 2, 40),
        BeamParams(4500, 100, 1, 2, 40),
        BeamParams(4500, 100, 0, 3, 40),
        BeamParams(8000, 100, 0, 2, 40),
        BeamParams(4500, 180, 0, 2, 40),
        BeamParams(3000, 100, 0, 3, 80),
    ]

    return {
        "keep_first": unique_params([*keep, *width, *bad, *expand, *depth]),
        "width_first": unique_params([*width, *keep, *expand, *bad, *depth]),
        "bad_first": unique_params([*bad, *keep, *width, *expand, *depth]),
        "mixed_light": unique_params(mixed),
    }


def make_schedules() -> list[Schedule]:
    penalty_variants = {
        "p10_50": (10, 50),
        "p0_10_50_100": (0, 10, 50, 100),
        "p10_30_50_100": (10, 30, 50, 100),
    }
    schedules: list[Schedule] = []
    for order_name, params in candidate_orders().items():
        for penalty_name, penalties in penalty_variants.items():
            schedules.append(
                Schedule(
                    name=f"{order_name}__{penalty_name}",
                    params=params,
                    penalties=penalties,
                )
            )
    return schedules


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Generate temporary v201 variants with a 1.95s in-solver time loop, "
            "then evaluate parameter schedules without touching v201 or results CSVs."
        )
    )
    parser.add_argument(
        "input_dir",
        nargs="?",
        default=str(DEFAULT_INPUT_DIR),
        help="input directory (default: tools/in)",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=1,
        help="parallel case jobs per schedule (default: 1; use 1 for contest-like evaluation)",
    )
    parser.add_argument(
        "--smoke",
        action="store_true",
        help="build one generated solver and evaluate only tools/in/0000.txt",
    )
    parser.add_argument(
        "--stage1-step",
        type=int,
        default=5,
        help="coarse stage uses every Nth sorted case (default: 5 -> 20/100 cases)",
    )
    parser.add_argument(
        "--topk",
        type=int,
        default=5,
        help="number of generated schedules promoted to full evaluation (default: 5)",
    )
    parser.add_argument(
        "--limit-schedules",
        type=int,
        default=0,
        help="debug knob: evaluate only the first N generated schedules when >0",
    )
    parser.add_argument(
        "--keep-generated",
        action="store_true",
        help="do not delete previous target/param_tune_v201 contents before running",
    )
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be >= 1")
    if args.stage1_step < 1:
        parser.error("--stage1-step must be >= 1")
    if args.topk < 1:
        parser.error("--topk must be >= 1")
    return args


def list_cases(input_dir: Path) -> list[Path]:
    cases = sorted(p for p in input_dir.rglob("*") if p.is_file())
    if not cases:
        raise SystemExit(f"error: no input files: {input_dir}")
    return cases


def parse_score(stdout: str) -> int | None:
    tokens = stdout.split()
    if not tokens:
        return None
    try:
        return int(tokens[-1])
    except ValueError:
        return None


def run_checked(command: list[str], *, cwd: Path = ROOT) -> None:
    result = subprocess.run(command, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def ensure_binaries() -> None:
    if not BASELINE_BIN.is_file():
        eprint("build: baseline v201pro_hybrid")
        run_checked(["cargo", "build", "--release", "--quiet", "--bin", "v201pro_hybrid"])
    if not SCORER_BIN.is_file():
        eprint("build: tools vis")
        run_checked(
            [
                "cargo",
                "build",
                "--release",
                "--quiet",
                "--manifest-path",
                str(ROOT / "tools" / "Cargo.toml"),
                "--bin",
                "vis",
            ]
        )


def replace_block(source: str, start_marker: str, end_marker: str, replacement: str) -> str:
    start = source.index(start_marker)
    end = source.index(end_marker, start)
    return source[:start] + replacement.rstrip() + "\n\n" + source[end:]


def generated_solver_source(schedule: Schedule) -> str:
    source = SOURCE.read_text(encoding="utf-8")
    source = source.replace(
        "use std::io::{Read, Write};",
        "use std::io::{Read, Write};\nuse std::time::Instant;",
        1,
    )

    params = ",\n    ".join(p.rust() for p in schedule.params)
    penalties = ", ".join(str(p) for p in schedule.penalties)

    route_replacement = f"""
#[derive(Clone, Copy, Debug)]
struct BeamParams {{
    width: usize,
    max_expand: usize,
    bad_weight: i32,
    extra_depth: i32,
    final_keep: usize,
}}

const PARAM_SCHEDULE: &[BeamParams] = &[
    {params}
];
const GREEDY_PENALTIES: &[i32] = &[{penalties}];

fn update_best(best: &mut CandidateSolution, sol: CandidateSolution) {{
    if sol.turn_count < best.turn_count {{
        *best = sol;
    }}
}}

fn solve_route_strategy_with_params(
    init: &[Vec<usize>],
    params: BeamParams,
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> CandidateSolution {{
    let db = DistBuilder::new(init);
    let mut nodes = Vec::new();
    let final_ids = db.beam_final_nodes(
        &mut nodes,
        params.width,
        params.max_expand,
        params.bad_weight,
        params.extra_depth,
        params.final_keep,
    );

    let mut best = CandidateSolution::empty();
    for id in final_ids {{
        update_best(&mut best, make_solution_from_distribution(&db, &nodes, id, init, memo));
    }}
    best
}}

fn solve_greedy_penalty(
    init: &[Vec<usize>],
    pen: i32,
    memo: &mut HashMap<usize, Vec<AbsOp>>,
) -> CandidateSolution {{
    let db = DistBuilder::new(init);
    let mut nodes = Vec::new();
    let id = db.greedy_final_node(&mut nodes, pen);
    make_solution_from_distribution(&db, &nodes, id, init, memo)
}}
"""
    source = replace_block(
        source,
        "fn solve_route_strategy(",
        "fn old_distribute_to_route_sidings",
        route_replacement,
    )

    solve_replacement = """
fn solve(init: &[Vec<usize>]) -> CandidateSolution {
    let start = Instant::now();
    let mut memo = HashMap::new();
    let mut best = CandidateSolution::empty();

    let baseline = BeamParams {
        width: 3000,
        max_expand: 100,
        bad_weight: 0,
        extra_depth: 2,
        final_keep: 12,
    };
    update_best(
        &mut best,
        solve_route_strategy_with_params(init, baseline, &mut memo),
    );
    update_best(&mut best, solve_old_fallback(init));

    for &pen in &[10, 50] {
        update_best(&mut best, solve_greedy_penalty(init, pen, &mut memo));
    }
    for &pen in GREEDY_PENALTIES {
        if pen != 10 && pen != 50 {
            update_best(&mut best, solve_greedy_penalty(init, pen, &mut memo));
        }
    }

    let mut idx = 0usize;
    while idx < PARAM_SCHEDULE.len() && start.elapsed().as_secs_f64() < TIME_LIMIT_SEC {
        let params = PARAM_SCHEDULE[idx];
        idx += 1;
        update_best(
            &mut best,
            solve_route_strategy_with_params(init, params, &mut memo),
        );
    }

    assert!(best.turn_count <= MAX_TURNS);
    best
}
"""
    source = replace_block(source, "fn solve(init: &[Vec<usize>]) -> CandidateSolution", "fn read_input()", solve_replacement)
    source = re.sub(r"^// v201pro_hybrid\.rs", f"// generated_{schedule.name}.rs", source, count=1)
    return source


def prepare_workspace(keep_generated: bool) -> None:
    if WORK_DIR.exists() and not keep_generated:
        shutil.rmtree(WORK_DIR)
    GEN_DIR.mkdir(parents=True, exist_ok=True)
    BIN_DIR.mkdir(parents=True, exist_ok=True)
    OUT_DIR.mkdir(parents=True, exist_ok=True)


def build_schedule(schedule: Schedule) -> Path:
    src_path = GEN_DIR / f"{schedule.name}.rs"
    bin_path = BIN_DIR / schedule.name
    src_path.write_text(generated_solver_source(schedule), encoding="utf-8")
    eprint(f"build: {schedule.name}")
    run_checked(
        [
            "rustc",
            "--edition=2024",
            "-O",
            str(src_path),
            "-o",
            str(bin_path),
        ]
    )
    return bin_path


def run_case(binary: Path, case_path: Path, output_dir: Path) -> CaseResult:
    case_name = case_path.name
    output_path = output_dir / case_name
    err_path = output_dir / f"{case_name}.err"
    output_dir.mkdir(parents=True, exist_ok=True)

    start_ns = time.monotonic_ns()
    with case_path.open("rb") as fin, output_path.open("wb") as fout, err_path.open("wb") as ferr:
        proc = subprocess.run([str(binary)], stdin=fin, stdout=fout, stderr=ferr)
    elapsed_ms = (time.monotonic_ns() - start_ns) // 1_000_000
    if proc.returncode != 0:
        return CaseResult(case_name, None, int(elapsed_ms), "run_fail")

    with err_path.open("ab") as ferr:
        score_proc = subprocess.run(
            [str(SCORER_BIN), str(case_path), str(output_path)],
            stdout=subprocess.PIPE,
            stderr=ferr,
            text=True,
        )
    if score_proc.returncode != 0:
        return CaseResult(case_name, None, int(elapsed_ms), "score_fail")
    score = parse_score(score_proc.stdout)
    if score is None:
        return CaseResult(case_name, None, int(elapsed_ms), "score_parse_fail")
    return CaseResult(case_name, score, int(elapsed_ms), "ok")


def evaluate_binary(name: str, binary: Path, cases: list[Path], jobs: int) -> ScheduleResult:
    eprint(f"eval: {name} cases={len(cases)} jobs={jobs}")
    output_dir = OUT_DIR / name
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    if jobs == 1:
        results = [run_case(binary, case, output_dir) for case in cases]
    else:
        with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as executor:
            futures = [executor.submit(run_case, binary, case, output_dir) for case in cases]
            results = [future.result() for future in futures]

    ok = [r for r in results if r.status == "ok" and r.score is not None]
    failures = len(results) - len(ok)
    if ok:
        total = sum(r.score for r in ok if r.score is not None)
        avg_score = round(total / len(ok))
        min_score = min(r.score for r in ok if r.score is not None)
        max_score = max(r.score for r in ok if r.score is not None)
        avg_elapsed = round(sum(r.elapsed_ms for r in ok) / len(ok))
        max_elapsed = max(r.elapsed_ms for r in ok)
    else:
        avg_score = min_score = max_score = avg_elapsed = max_elapsed = 0
    return ScheduleResult(
        name=name,
        cases=len(results),
        failures=failures,
        avg_score=avg_score,
        min_score=min_score,
        max_score=max_score,
        avg_elapsed_ms=avg_elapsed,
        max_elapsed_ms=max_elapsed,
        bin_path=binary,
    )


def print_table(title: str, results: list[ScheduleResult]) -> None:
    print()
    print(title)
    print(
        "name,cases,failures,avg_score,min_score,max_score,avg_elapsed_ms,max_elapsed_ms"
    )
    for r in sorted(results, key=lambda x: (x.failures, -x.avg_score, x.avg_elapsed_ms, x.name)):
        print(
            f"{r.name},{r.cases},{r.failures},{r.avg_score},{r.min_score},"
            f"{r.max_score},{r.avg_elapsed_ms},{r.max_elapsed_ms}"
        )


def main() -> int:
    args = parse_args()
    input_dir = Path(args.input_dir).resolve()
    cases = list_cases(input_dir)
    stage1_cases = cases[:1] if args.smoke else cases[:: args.stage1_step]
    full_cases = cases[:1] if args.smoke else cases

    schedules = make_schedules()
    if args.limit_schedules > 0:
        schedules = schedules[: args.limit_schedules]
    if args.smoke:
        schedules = schedules[:1]

    prepare_workspace(args.keep_generated)
    ensure_binaries()

    baseline_stage1 = evaluate_binary("baseline_v201", BASELINE_BIN, stage1_cases, args.jobs)
    schedule_bins: dict[str, Path] = {}
    stage1_results = [baseline_stage1]

    for schedule in schedules:
        bin_path = build_schedule(schedule)
        schedule_bins[schedule.name] = bin_path
        stage1_results.append(evaluate_binary(schedule.name, bin_path, stage1_cases, args.jobs))

    print_table("stage1", stage1_results)

    promoted = [
        r
        for r in sorted(stage1_results, key=lambda x: (x.failures, -x.avg_score, x.avg_elapsed_ms, x.name))
        if r.name != "baseline_v201"
    ][: args.topk]
    full_names = ["baseline_v201", *[r.name for r in promoted]]

    full_results = []
    for name in full_names:
        binary = BASELINE_BIN if name == "baseline_v201" else schedule_bins[name]
        full_results.append(evaluate_binary(name, binary, full_cases, args.jobs))

    print_table("full", full_results)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
