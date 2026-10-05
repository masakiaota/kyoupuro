#!/usr/bin/env python3
"""Build and fully preprocess both variants before any reconstruction is run."""
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v029_audit"
PARENT = "v028_two_order_lns"
CHILD = "v029_selective_two_order_lns"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
MODES = {"local": ["-DLOCAL"], "production": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}


def run(args):
    result = subprocess.run(args, capture_output=True, text=True, cwd=ROOT, check=True)
    if result.stderr:
        raise RuntimeError(result.stderr)
    return result.stdout


def main():
    OUT.mkdir(exist_ok=True)
    parent = ROOT / f"src/bin/{PARENT}.cpp"
    child = ROOT / f"src/bin/{CHILD}.cpp"
    hashes = {"parent_sha256": hashlib.sha256(parent.read_bytes()).hexdigest(),
              "solver_sha256": hashlib.sha256(child.read_bytes()).hexdigest()}
    assert hashes["parent_sha256"] == "29aa1b31304a06d63db75ad152be6a9e52c2aa752c7e8d7b516b506e9334e717"
    report = {**hashes, "compiler": run(["g++-15", "--version"]).splitlines()[0],
              "sdk": os.environ.get("SDKROOT"), "deployment_target": os.environ.get("MACOSX_DEPLOYMENT_TARGET")}
    (OUT / "source.diff").write_text("".join(difflib.unified_diff(
        parent.read_text().splitlines(True), child.read_text().splitlines(True), fromfile=PARENT, tofile=CHILD)))
    for mode, defines in MODES.items():
        build = ["./scripts/build_solver.sh", *(["--no-local"] if mode == "production" else []), CHILD]
        (OUT / f"build_{mode}.log").write_text(run(build))
        shutil.copy2(ROOT / "target/release" / CHILD, OUT / f"{CHILD}_{mode}")
        expanded = []
        for name, path in [(PARENT, parent), (CHILD, child)]:
            source = run(["g++-15", *FLAGS, *defines, "-E", "-P", str(path)])
            (OUT / f"{name}_{mode}.ii").write_text(source)
            source = source.replace(PARENT, "solver").replace(CHILD, "solver")
            # macOS assert expands to __assert_rtn(function, file, line, expr).
            source = re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)
            expanded.append(source)
        diff = "".join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True),
                                         fromfile=PARENT, tofile=CHILD))
        (OUT / f"{mode}_preprocessed.diff").write_text(diff)
        report[mode] = {"diff_lines": len(diff.splitlines()), "hunks": diff.count("\n@@ ")}
        print(mode, "build and preprocessing complete", flush=True)
    (OUT / "check_build.log").write_text(run(["./scripts/build_solver.sh", "check_v029_selective_two_orders"]))
    report["input_sha256"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                              for p in sorted((ROOT / "tools/in").glob("*.txt"))}
    (OUT / "static_verification.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps({k: v for k, v in report.items() if k != "input_sha256"}, indent=2))


if __name__ == "__main__":
    main()
