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
OUT = ROOT / "adhoc/v052_audit"
PARENT = "v050_joint_towers"
CHILD = "v052_finite_cooperation"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
MODES = {"local": ["-DLOCAL"], "production": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}


def run(args):
    result = subprocess.run(args, capture_output=True, text=True, cwd=ROOT)
    if result.returncode:
        raise RuntimeError(f"{args}:\n{result.stderr}")
    if result.stderr:
        raise RuntimeError(result.stderr)
    return result.stdout


def main():
    OUT.mkdir(exist_ok=True)
    parent = ROOT / f"src/bin/{PARENT}.cpp"
    child = ROOT / f"src/bin/{CHILD}.cpp"
    hashes = {"parent_sha256": hashlib.sha256(parent.read_bytes()).hexdigest(),
              "solver_sha256": hashlib.sha256(child.read_bytes()).hexdigest()}
    assert hashes["parent_sha256"] == "22d846f1a4dacc816c4634fdc2755af13f5f329ca5398bf21abbc695e670c421"
    hashes["planner_parent_sha256"] = hashlib.sha256((ROOT / "src/bin/v043_cooperative_events.cpp").read_bytes()).hexdigest()
    assert hashes["planner_parent_sha256"] == "9f183da8812c5f186fb224005e7ba0cad1ef4c8e2116c851455544205b4e8727"
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
        projected = expanded[1]
        start = projected.index("class FiniteRouter {")
        end = projected.index("struct IdentityBoard {", start)
        projected = projected[:start] + projected[end:]
        start = projected.index("    const size_t pre_finite_ops=")
        end = projected.index("    try {\n        Board check=geo.initial;", start)
        projected = projected[:start] + projected[end:]
        projected = projected.replace('<<" pre_finite="<<pre_finite_ops', '')
        if mode == "production":
            projected = projected.replace("constexpr double JUDGE_TIME_LIMIT_SEC = 1.90;\nconstexpr double PROGRAM_TIME_LIMIT_SEC = JUDGE_TIME_LIMIT_SEC;\n", "", 1)
        normalized = lambda text: "\n".join(line for line in text.splitlines() if line.strip())
        assert normalized(projected) == normalized(expanded[0]), "unexpected parent behavior change"
        report[mode]["unchanged_parent_after_removing_additions"] = True
        print(mode, "build and preprocessing complete", flush=True)
    (OUT / "check_build.log").write_text(run(["./scripts/build_solver.sh", "check_v052_finite_cooperation"]))
    report["input_sha256"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                              for p in sorted((ROOT / "tools/in").glob("*.txt"))}
    report["parent_output_sha256"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                     for p in sorted((ROOT / "results/out" / PARENT).glob("*.txt"))}
    (OUT / "static_verification.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps({k: v for k, v in report.items() if k not in ("input_sha256", "parent_output_sha256")}, indent=2))


if __name__ == "__main__":
    main()
