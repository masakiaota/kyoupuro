#!/usr/bin/env python3
import argparse
import csv
import json
import os
import subprocess
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCORE_DETAIL = ROOT / "results" / "score_detail.csv"
TOOLS_IN = ROOT / "tools" / "in"
TARGET = ROOT / "target" / "release"
DEFAULT_BINS = ["v123pro_merge005and122", "v125_faster123"]
KEYS = [
    "final_turns",
    "solve_base_iters",
    "solve_base_beam_inputs",
    "solve_base_new_map_size_sum",
    "solve_base_rescue_calls",
    "fastlane_calls",
    "fastlane_success",
    "plan_quick_calls",
    "plan_quick_expansions",
    "plan_quick_success",
    "stage_search_calls",
    "stage_search_expansions",
    "stage_search_solution_hits",
    "collect_exact_calls",
    "collect_exact_targets",
    "collect_exact_returned",
    "collect_exact_turn_calls",
    "collect_exact_turn_targets",
    "collect_exact_turn_returned",
    "try_target_exact_calls",
    "try_target_exact_success",
    "try_target_empty_path_calls",
    "try_target_empty_path_expansions",
    "try_target_empty_path_success",
    "navigate_safe_calls",
    "navigate_safe_steps",
    "navigate_safe_success",
    "navigate_loose_calls",
    "navigate_loose_steps",
    "navigate_loose_success",
    "shrink_calls",
    "shrink_steps",
    "shrink_success",
    "bfs_calls",
    "bfs_pops",
    "bfs_success",
    "try_recover_exact_calls",
    "try_recover_exact_steps",
    "try_recover_exact_success",
    "encode_key_calls",
    "step_calls",
    "step_ate",
    "step_bite",
    "solve_suffix_iters",
    "solve_suffix_beam_inputs",
    "solve_suffix_new_map_size_sum",
    "optimize_suffix_windows",
    "time_over_hits",
]


def read_latest_rows():
    rows = list(csv.DictReader(SCORE_DETAIL.open()))
    latest = {}
    for row in rows:
        latest[row["bin"]] = row
    return latest


def pick_cases(top_k: int):
    latest = read_latest_rows()
    base = latest["v123pro_merge005and122"]
    cand = latest["v125_faster123"]
    diffs = []
    for i in range(100):
        key = f"case{i:04d}"
        a = int(base[key])
        b = int(cand[key])
        diffs.append((b - a, f"{i:04d}.txt"))
    diffs.sort(reverse=True)
    cases = ["0000.txt"]
    for _, case in diffs[:top_k]:
        if case not in cases:
            cases.append(case)
    return cases


def build(bin_name: str):
    env = os.environ.copy()
    rustflags = env.get("RUSTFLAGS", "")
    extra = "--cfg profile_counts -Aunexpected_cfgs"
    env["RUSTFLAGS"] = f"{rustflags} {extra}".strip()
    cmd = ["cargo", "build", "--release", "--bin", bin_name]
    subprocess.run(cmd, cwd=ROOT, env=env, check=True)


def parse_profile(stderr: str):
    out = {}
    for line in stderr.splitlines():
        if not line.startswith("PROFILE\t"):
            continue
        _, key, value = line.split("\t", 2)
        if key == "bin":
            out["bin"] = value
        else:
            out[key] = int(value)
    return out


def run_case(bin_name: str, case_name: str, repeats: int):
    inp = (TOOLS_IN / case_name).read_bytes()
    totals = {}
    elapsed_total = 0.0
    stdout_lines_total = 0
    for _ in range(repeats):
        t0 = time.perf_counter()
        proc = subprocess.run(
            [str(TARGET / bin_name)],
            input=inp,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=ROOT,
            check=True,
        )
        elapsed_total += time.perf_counter() - t0
        stdout_lines_total += len(proc.stdout.splitlines())
        prof = parse_profile(proc.stderr.decode())
        for key, value in prof.items():
            if key == "bin":
                totals[key] = value
            else:
                totals[key] = totals.get(key, 0) + value
    out = {"bin": bin_name}
    for key, value in totals.items():
        if key == "bin":
            continue
        out[key] = round(value / repeats)
    out["elapsed_ms"] = round(elapsed_total * 1000 / repeats, 3)
    out["stdout_lines"] = round(stdout_lines_total / repeats)
    return out


def ratio(num, den):
    if den == 0:
        return None
    return num / den


def summarize_pair(case_name, a, b):
    summary = {
        "case": case_name,
        "v123_elapsed_ms": a["elapsed_ms"],
        "v125_elapsed_ms": b["elapsed_ms"],
        "elapsed_ratio": ratio(b["elapsed_ms"], a["elapsed_ms"]),
        "keys": {},
    }
    for key in KEYS:
        av = a.get(key, 0)
        bv = b.get(key, 0)
        summary["keys"][key] = {
            "v123": av,
            "v125": bv,
            "ratio": ratio(bv, av),
            "delta": bv - av,
        }
    return summary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", nargs="*", help="case names like 0015.txt")
    parser.add_argument("--top-k", type=int, default=5)
    parser.add_argument(
        "--json-out",
        default=str(ROOT / "results" / "profile_v123_v125_counts.json"),
    )
    parser.add_argument("--repeats", type=int, default=1)
    args = parser.parse_args()

    cases = args.cases or pick_cases(args.top_k)
    latest = read_latest_rows()

    for bin_name in DEFAULT_BINS:
        build(bin_name)

    payload = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "cases": [],
    }
    for case_name in cases:
        case_key = f"case{case_name[:4]}"
        prof_a = run_case(DEFAULT_BINS[0], case_name, args.repeats)
        prof_b = run_case(DEFAULT_BINS[1], case_name, args.repeats)
        row = summarize_pair(case_name, prof_a, prof_b)
        row["score_v123"] = int(latest[DEFAULT_BINS[0]][case_key])
        row["score_v125"] = int(latest[DEFAULT_BINS[1]][case_key])
        row["score_delta"] = row["score_v125"] - row["score_v123"]
        payload["cases"].append(row)

    out_path = Path(args.json_out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=2, sort_keys=True))

    print(f"wrote {out_path}")
    for case in payload["cases"]:
        print(
            f"{case['case']}: score_delta={case['score_delta']} "
            f"elapsed_ratio={case['elapsed_ratio']:.3f} "
            f"stage_search_expansions_ratio={case['keys']['stage_search_expansions']['ratio']:.3f} "
            f"collect_exact_calls_ratio={case['keys']['collect_exact_calls']['ratio']:.3f} "
            f"bfs_calls_ratio={case['keys']['bfs_calls']['ratio']:.3f}"
        )


if __name__ == "__main__":
    sys.exit(main())
