#!/usr/bin/env python3
"""Run the frozen equivalence checks and the preregistered fixed-work comparison."""
import csv
import fcntl
import hashlib
import json
from pathlib import Path
import re
import statistics
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v033_capacity"
CAPACITY_KEYS = {"floor_cells", "cell_capacity", "board_bytes", "identity_board_bytes", "geometry_bytes",
                 "floor_dist_bytes", "group_bytes", "router_bytes", "portion_router_bytes",
                 "constructor_bytes", "temporal_lns_bytes", "weight_bytes"}
COUNT = re.compile(r"\[summary.count\] ([^=]+)=(-?\d+)")


def unchanged():
    report = json.loads((OUT/"static_verification.json").read_text())
    for name, expected in report["sources"].items():
        actual = hashlib.sha256((ROOT/name).read_bytes()).hexdigest()
        if actual != expected:
            raise RuntimeError("Changed: "+name)


def run_fixed():
    records = []
    for folder in ("parent", "child"):
        (OUT/"fixed_clock"/folder).mkdir(parents=True, exist_ok=True)
    for index, path in enumerate(sorted((ROOT/"tools/in").glob("*.txt"))):
        runs = []
        for mode in ("parent", "child"):
            result = subprocess.run([str(OUT/f"{mode}_fixed_clock")], input=path.read_bytes(),
                                    capture_output=True, timeout=60, cwd=ROOT)
            (OUT/"fixed_clock"/mode/path.name).write_bytes(result.stdout)
            (OUT/"fixed_clock"/mode/(path.name+".err")).write_bytes(result.stderr)
            if result.returncode:
                raise RuntimeError(f"{path.name} {mode} exit={result.returncode}: {result.stderr[-2000:]!r}")
            counts = {k: int(v) for k,v in COUNT.findall(result.stderr.decode()) if k not in CAPACITY_KEYS}
            rng = re.search(rb"\[fixed.rng\] (\d+)", result.stderr).group(1)
            ticks = re.search(rb"\[fixed.ticks\] (\d+)", result.stderr).group(1)
            runs.append((result.stdout, counts, rng, ticks))
        if runs[0] != runs[1]:
            reasons = [name for name,a,b in zip(("output", "counts", "rng", "ticks"), runs[0], runs[1]) if a!=b]
            (OUT/"fixed_clock_failure.json").write_text(json.dumps({"case": path.name, "different": reasons,
                "parent_counts": runs[0][1], "child_counts": runs[1][1]}, indent=2)+"\n")
            raise RuntimeError(f"Fixed-clock mismatch {path.name}: {reasons}")
        records.append({"case": path.name, "T": runs[0][1]["T"], "ticks": int(runs[0][3]),
                        "rng": int(runs[0][2]), "sha256": hashlib.sha256(runs[0][0]).hexdigest(),
                        "count_keys": len(runs[0][1])})
        if (index+1)%10==0:
            print(f"fixed_clock verified={index+1}", flush=True)
    unchanged()
    (OUT/"fixed_clock_summary.json").write_text(json.dumps({"verified_cases": len(records),
        "output_equal": True, "all_shared_counts_equal": True, "rng_equal": True, "ticks_equal": True,
        "child_ubsan_trap": True, "cases": records}, indent=2)+"\n")


def stream(args, name):
    with (OUT/name).open("w") as log:
        process = subprocess.Popen(args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        for line in process.stdout:
            log.write(line);log.flush();print(line, end="", flush=True)
        if process.wait():
            raise RuntimeError(f"Failed: {args}; see {name}")


def summarize():
    rows = list(csv.DictReader((OUT/"fixed_work_samples.csv").open()))
    result = {}
    for group, predicate in (("all_100", lambda name: True), ("generated_99", lambda name: name!="0000.txt"),
                             ("case0000", lambda name: name=="0000.txt")):
        group_result = {}
        for metric in ("build_ns", "reconstruct_ns", "smooth_ns", "total_ns"):
            times = {name: [sum(int(r[metric]) for r in rows if int(r["round"])==round and r["variant"]==name and predicate(r["case"]))
                           for round in range(1,6)] for name in ("v028", "v033")}
            old, new = (statistics.median(times[name]) for name in ("v028", "v033"))
            shorter = sum(b<a for a,b in zip(times["v028"],times["v033"]))
            group_result[metric] = {"round_totals_ns": times, "v028_median_ns": old, "v033_median_ns": new,
                                   "change_percent": 100*(new/old-1), "v033_shorter_rounds": shorter,
                                   "speed_improved": new<=old*0.97 and shorter>=4}
        result[group] = group_result
    result["verified_rows"] = len(rows)
    result["method"] = "1 warmup round excluded, 5 measured rounds; CPU time; same frozen candidates"
    result["sources_unchanged"] = True
    (OUT/"fixed_work_summary.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps(result["generated_99"], indent=2))


def main():
    mode = sys.argv[1]
    unchanged()
    print("Waiting for evaluation lock", flush=True)
    with (ROOT/"results/.eval.lock").open("a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if mode == "fixed":
            run_fixed()
        elif mode == "bench":
            executable = str(ROOT/"target/release/bench_v033_capacity")
            stream([executable, "--check", str(ROOT), str(OUT)], "check_capacity.log")
            stream([executable, "--measure", str(ROOT), str(OUT)], "fixed_work.log")
            unchanged();summarize()
        else:
            raise ValueError(mode)
        fcntl.flock(lock, fcntl.LOCK_UN)


if __name__ == "__main__":
    main()
