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
OUT = ROOT / "adhoc/v054_audit"
PARENT = "v050_joint_towers"
CHILD = "v054_route_slack"
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
    (OUT / "check_build.log").write_text(run(["./scripts/build_solver.sh", "check_v054_route_slack"]))
    registered = child.read_text().replace("// v054_route_slack.cpp", "// v050_joint_towers.cpp", 1)
    a = registered.index("struct CoupledRoutesStats {")
    b = registered.index("// Between two cells without home returns,", a)
    registered = registered[:a] + registered[b:]
    registered = registered.replace("constexpr double JUDGE_TIME_LIMIT_SEC = 1.90;\nconstexpr double LOCAL_TIME_RATIO = 1.0;\nconstexpr double PROGRAM_TIME_LIMIT_SEC = JUDGE_TIME_LIMIT_SEC;\n", "", 1)
    a = registered.index("    const size_t pre_coupled_ops=")
    b = registered.index("    try {\n        Board check=geo.initial;", a)
    registered = registered[:a] + registered[b:]
    assert registered == parent.read_text()
    report["unchanged_parent_after_removing_registered_additions"] = True
    report["helper_sha256"] = hashlib.sha256((ROOT / "adhoc/bin/check_v054_route_slack.cpp").read_bytes()).hexdigest()
    report["route_parent_sha256"] = hashlib.sha256((ROOT / "src/bin/v049_coupled_routes.cpp").read_bytes()).hexdigest()
    report["input_sha256"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                              for p in sorted((ROOT / "tools/in").glob("*.txt"))}
    plans = list((ROOT / "results/out/v050_joint_towers").glob("*.txt"))
    long = ROOT / "results/long_search/v047/20260929T005348_ca87d7da/cases"
    plans += list(long.glob("*/seeds/00.txt")) + list(long.glob("*/continuous/round_0000/search/before/*.txt"))
    assert len(plans) == 495
    report["plan_sha256"] = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(plans)}
    (OUT / "static_verification.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps({k: v for k, v in report.items() if k not in ("input_sha256", "plan_sha256")}, indent=2))


if __name__ == "__main__":
    main()
