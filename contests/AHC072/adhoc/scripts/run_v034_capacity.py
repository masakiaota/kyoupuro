#!/usr/bin/env python3
"""Build and run the preregistered capacity comparison once."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v034_capacity_factors"
NAME = "bench_v034_capacity_factors"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]


def unchanged():
    manifest = json.loads((OUT / "manifest.json").read_text())
    for section in ("originals", "frozen"):
        for name, expected in manifest[section].items():
            if hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != expected:
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


def build():
    env = os.environ.copy()
    env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
    env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    for mode, options, defines in (("local", [], ["-DLOCAL"]),
                                  ("production", ["--no-local"], ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"])):
        print("Building", mode, flush=True)
        stream([str(ROOT / "scripts/build_solver.sh"), *options, NAME], f"build_{mode}.log", env)
        shutil.copy2(ROOT / "target/release" / NAME, OUT / f"bench_{mode}")
        with (OUT / f"{mode}.ii").open("w") as preprocessed:
            subprocess.run(["g++-15", *FLAGS, *defines, "-E", "-P",
                            str(ROOT / "adhoc/bin" / (NAME + ".cpp"))], cwd=ROOT, env=env,
                           stdout=preprocessed, check=True)
    (OUT / "build_environment.json").write_text(json.dumps({
        "flags": FLAGS, "SDKROOT": env["SDKROOT"],
        "MACOSX_DEPLOYMENT_TARGET": env["MACOSX_DEPLOYMENT_TARGET"]}, indent=2) + "\n")


def main():
    mode = sys.argv[1]
    unchanged()
    if mode == "build":
        build()
    elif mode in ("check", "measure"):
        if mode == "measure" and (OUT / "fixed_work_samples.csv").exists():
            raise RuntimeError("The planned measurement has already been started")
        print("Waiting for evaluation lock", flush=True)
        with (ROOT / "results/.eval.lock").open("a+") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            binary = OUT / ("bench_local" if mode == "check" else "bench_production")
            stream([str(binary), "--check" if mode == "check" else "--measure",
                    str(OUT / "fixture"), str(OUT)], f"{mode}.log")
    else:
        raise ValueError(mode)
    unchanged()
    print("Original and frozen files unchanged", flush=True)


if __name__ == "__main__":
    main()
