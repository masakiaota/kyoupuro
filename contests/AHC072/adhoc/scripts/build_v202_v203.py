#!/usr/bin/env python3
"""ビルドと完全な前処理照合、評価前の記録固定。solverは実行しない。"""
from concurrent.futures import ThreadPoolExecutor
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from make_v202_v203 import PAIRS

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v202_v203_audit"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
MODES = {"local": ["-DLOCAL"], "production": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}
PARENT_RUNS = {
    "v076_relative_tuned": "20260930T202705+0900_v076_relative_tuned_e688d5",
    "v201_floor_nn_selector": "20261002T204208+0900_v201_floor_nn_selector_0a0ca3",
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def main():
    assert not (AUDIT / "preflight.json").exists(), "already frozen"
    env = os.environ.copy()
    if sys.platform == "darwin":
        env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
        env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
    compiler = env.get("CXX", "g++-15")

    def command(args, **kwargs):
        p = subprocess.run([compiler, *args], cwd=ROOT, env=env, text=True,
                           capture_output=True, check=True, **kwargs)
        return p.stdout, p.stderr

    def normalize(source):
        for pair in PAIRS:
            for name in pair:
                source = source.replace(str(ROOT / f"src/bin/{name}.cpp"), "solver.cpp")
                source = source.replace(f'"{name}.cpp"', '"solver.cpp"')
        source = source.replace('"<stdin>"', '"solver.cpp"')
        return re.sub(r'("solver.cpp",\s*)\d+', r'\g<1>0', source)

    def build(job):
        parent, child, mode = job
        flags = [*FLAGS, *MODES[mode]]
        source = ROOT / f"src/bin/{child}.cpp"
        binary = AUDIT / f"{child}_{mode}"
        _, warnings = command([*flags, str(source), "-o", str(binary)])
        (AUDIT / f"{child}_{mode}_build.log").write_text(warnings)
        restored = source.read_text()
        changes = json.loads((AUDIT / f"{child}_changes.json").read_text())
        for change in reversed(changes):
            at, added = change["offset"], change["new"]
            assert restored[at:at + len(added)] == added
            restored = restored[:at] + change["old"] + restored[at + len(added):]
        assert restored == (ROOT / f"src/bin/{parent}.cpp").read_text()
        before, _ = command([*flags, "-E", "-P", str(ROOT / f"src/bin/{parent}.cpp")])
        after, _ = command([*flags, "-E", "-P", str(source)])
        undo, _ = command([*flags, "-E", "-P", "-x", "c++", "-"], input=restored)
        before, after, undo = map(normalize, (before, after, undo))
        assert before == undo, "restored complete preprocessing differs"
        (AUDIT / f"{child}_{mode}.ii").write_text(after)
        diff = "".join(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
                                         fromfile=parent, tofile=child))
        (AUDIT / f"{child}_{mode}.diff").write_text(diff)
        print(f"{child} {mode}: build and restored preprocessing passed", flush=True)
        return {"bin": child, "mode": mode, "binary_sha256": sha(binary),
                "diff_sha256": sha(AUDIT / f"{child}_{mode}.diff"),
                "warnings": warnings, "registered_changes": len(changes)}

    jobs = [(parent, child, mode) for parent, child in PAIRS for mode in MODES]
    with ThreadPoolExecutor(max_workers=2) as pool:
        builds = list(pool.map(build, jobs))
    rows = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]
    parents = {name: [r for r in rows if r["run_id"] == run] for name, run in PARENT_RUNS.items()}
    cases = {p.name: sha(p) for p in sorted((ROOT / "tools/in").glob("*.txt"))}
    assert len(cases) == 100
    for name, group in parents.items():
        assert len(group) == 100 and {r["case_name"] for r in group} == set(cases)
        assert all(r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in" for r in group)
    assert not any(r["bin"] in [child for _, child in PAIRS] for r in rows)
    save(AUDIT / "preflight.json", {
        "solver_sha256": {name: sha(ROOT / f"src/bin/{name}.cpp") for pair in PAIRS for name in pair},
        "input_sha256": cases, "parent_records": parents, "builds": builds,
        "compiler": subprocess.check_output([compiler, "--version"], text=True).splitlines()[0],
        "jobs": 2, "local_time_ratio": 0.80, "solver_executions_at_freeze": 0,
    })
    print("Frozen both solvers and 100 inputs; no solver executed.", flush=True)


if __name__ == "__main__":
    main()
