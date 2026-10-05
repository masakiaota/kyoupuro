#!/usr/bin/env python3
"""Build, freeze, and run the preregistered v035 checks once."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from prepare_v035 import FLAGS, ROOT, OUT, digest

CAPACITY_KEYS = {"floor_cells", "cell_capacity", "board_bytes", "identity_board_bytes", "geometry_bytes"}
COUNT = re.compile(r"\[summary.count\] ([^=]+)=(-?\d+)")


def unchanged():
    manifest = json.loads((OUT / "frozen.json").read_text())
    for section in ("originals", "frozen"):
        for name, expected in manifest[section].items():
            if digest(ROOT / name) != expected:
                raise RuntimeError("Changed file: " + name)


def stream(args, name, env=None):
    with (OUT / name).open("w") as log:
        process = subprocess.Popen(args, cwd=ROOT, env=env, text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        for line in process.stdout:
            log.write(line)
            log.flush()
            print(line, end="", flush=True)
        if process.wait():
            raise RuntimeError(f"Command failed: {args}; see {name}")


def build(kind):
    if (OUT / "frozen.json").exists():
        raise RuntimeError("Frozen; do not rebuild diagnostics")
    env = os.environ.copy()
    env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
    env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    if kind == "bench":
        stream(["g++-15", *FLAGS, "-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX",
                str(ROOT / "adhoc/bin/bench_v035_tower_capacity.cpp"), "-o", str(OUT / "bench_production")],
               "build_bench.log", env)
    elif kind == "fixed":
        stream(["g++-15", *FLAGS, "-DLOCAL", "-fsanitize=undefined", "-fsanitize-undefined-trap-on-error",
                "-x", "c++", str(OUT / "child_fixed_clock.cpp.txt"), "-o", str(OUT / "child_fixed_clock")],
               "build_fixed.log", env)
    else:
        raise ValueError(kind)
    print("Built", kind, flush=True)


def freeze():
    if (OUT / "frozen.json").exists():
        raise RuntimeError("Already frozen")
    manifest = json.loads((OUT / "static_verification.json").read_text())
    paths = [ROOT / "src/bin/v035_tower_capacity_lns.cpp", ROOT / "adhoc/bin/bench_v035_tower_capacity.cpp"]
    paths += [ROOT / "adhoc/scripts" / name for name in
              ("make_v035.py", "prepare_v035.py", "run_v035_checks.py", "v035_bench_main.cpp.txt",
               "inspect_v035_assembly.py", "summarize_v035.py")]
    paths += sorted(p for p in OUT.rglob("*") if p.is_file())
    for name in ("local_solver", "production_solver", "bench_production", "child_fixed_clock", "assembly_summary.json"):
        if not (OUT / name).exists():
            raise RuntimeError("Missing preparation: " + name)
    manifest["frozen"] = {str(p.relative_to(ROOT)): digest(p) for p in paths}
    (OUT / "frozen.json").write_text(json.dumps(manifest, indent=2) + "\n")
    unchanged()
    print(f"Frozen {len(paths)} experiment files and {len(manifest['originals'])} original files", flush=True)


def parse_run(output, err):
    counts = {k: int(v) for k, v in COUNT.findall(err.decode()) if k not in CAPACITY_KEYS}
    rng = re.search(rb"\[fixed.rng\] (\d+)", err).group(1)
    ticks = re.search(rb"\[fixed.ticks\] (\d+)", err).group(1)
    return output, counts, rng, ticks


def fixed():
    destination = OUT / "fixed_clock/child"
    if destination.exists():
        raise RuntimeError("Fixed-clock run already started")
    destination.mkdir()
    records = []
    for index, path in enumerate(sorted((OUT / "fixture/tools/in").glob("*.txt"))):
        result = subprocess.run([str(OUT / "child_fixed_clock")], input=path.read_bytes(),
                                capture_output=True, timeout=60, cwd=ROOT)
        (destination / path.name).write_bytes(result.stdout)
        (destination / (path.name + ".err")).write_bytes(result.stderr)
        if result.returncode:
            raise RuntimeError(f"{path.name} exit={result.returncode}: {result.stderr[-2000:]!r}")
        old = OUT / "fixed_clock/parent" / path.name
        parent = parse_run(old.read_bytes(), old.with_name(old.name + ".err").read_bytes())
        child = parse_run(result.stdout, result.stderr)
        if parent != child:
            reasons = [name for name, a, b in zip(("output", "counts", "rng", "ticks"), parent, child) if a != b]
            (OUT / "fixed_clock_failure.json").write_text(json.dumps({"case": path.name, "different": reasons,
                "parent_counts": parent[1], "child_counts": child[1]}, indent=2) + "\n")
            raise RuntimeError(f"Fixed-clock mismatch {path.name}: {reasons}")
        records.append({"case": path.name, "T": child[1]["T"], "ticks": int(child[3]), "rng": int(child[2]),
                        "sha256": hashlib.sha256(child[0]).hexdigest(), "count_keys": len(child[1])})
        if (index + 1) % 10 == 0:
            print(f"fixed_clock verified={index + 1}", flush=True)
    (OUT / "fixed_clock_summary.json").write_text(json.dumps({"verified_cases": len(records),
        "output_equal": True, "all_shared_counts_equal": True, "rng_equal": True, "ticks_equal": True,
        "child_ubsan_trap": True, "cases": records}, indent=2) + "\n")


def main():
    mode = sys.argv[1]
    if mode in ("build-bench", "build-fixed"):
        build(mode.split("-")[1])
        return
    if mode == "freeze":
        freeze()
        return
    unchanged()
    print("Waiting for evaluation lock", flush=True)
    with (ROOT / "results/.eval.lock").open("a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if mode == "fixed":
            fixed()
        elif mode in ("check", "measure"):
            if mode == "measure" and (OUT / "fixed_work_samples.csv").exists():
                raise RuntimeError("Measurement already started")
            stream([str(OUT / "bench_production"), "--" + mode, str(OUT / "fixture"), str(OUT)], mode + ".log")
        else:
            raise ValueError(mode)
    unchanged()
    print("Original and frozen files unchanged", flush=True)


if __name__ == "__main__":
    main()
