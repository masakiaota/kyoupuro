#!/usr/bin/env python3
"""Freeze the capacity experiment and prepare independent diagnostic builds."""
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from prepare_v033 import transform

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v033_capacity"
PARENT = ROOT / "src/bin/v028_two_order_lns.cpp"
CHILD = ROOT / "src/bin/v033_capacity_lns.cpp"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
MODES = {"local": ["-DLOCAL"], "production": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}
CLOCK_BODY = "return chrono::duration<double>(chrono::steady_clock::now()-start).count();"


def run(args, log=None):
    r = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
    if log:
        (OUT / log).write_text(r.stdout+r.stderr)
    if r.returncode:
        raise RuntimeError(f"{args}: {r.stderr[:4000]}")
    if r.stderr:
        raise RuntimeError(f"Build warning: {r.stderr[:4000]}")
    return r.stdout


def prepare():
    parent, child = PARENT.read_text(), CHILD.read_text()
    assert child == transform(parent)
    assert hashlib.sha256(parent.encode()).hexdigest() == "29aa1b31304a06d63db75ad152be6a9e52c2aa752c7e8d7b516b506e9334e717"
    for name, source in (("parent", parent), ("child", child)):
        assert source.count(CLOCK_BODY) == 1
        fixed = source.replace("struct Clock {", "struct Clock {\n    mutable uint64_t ticks = 0;")
        fixed = fixed.replace(CLOCK_BODY, "return (++ticks) * 0.000100003;")
        fixed = fixed.replace("    for(auto m:best)cout", '''    cerr << "[fixed.rng] " << rng.x << "\\n[fixed.ticks] " << clk.ticks << '\\n';
    for(auto m:best)cout''')
        (OUT / f"{name}_fixed_clock.cpp.txt").write_text(fixed)
        exposed = source[:source.index("int main() {")]
        exposed = exposed.replace("class TemporalLNS {", "class TemporalLNS {\npublic:", 1)
        exposed = exposed.replace(CLOCK_BODY, "return 0.0;")
        (OUT / f"{name}_exposed.cpp.txt").write_text(exposed)
    paths = [PARENT, CHILD, ROOT/"src/bin/v000_template.cpp", ROOT/"notes/experiments/v033.md"]
    paths += sorted((ROOT/"tools/in").glob("*.txt"))
    paths += sorted((ROOT/"results/out/v028_two_order_lns").glob("*.txt"))
    record = {"sources": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
              "compiler": run(["g++-15", "--version"]).splitlines()[0],
              "sdk": os.environ.get("SDKROOT"), "deployment_target": os.environ.get("MACOSX_DEPLOYMENT_TARGET")}
    (OUT/"static_verification.json").write_text(json.dumps(record, indent=2)+"\n")
    for mode, defines in MODES.items():
        expanded = []
        for name, path in (("parent", PARENT), ("child", CHILD)):
            source = run(["g++-15", *FLAGS, *defines, "-E", "-P", str(path)])
            (OUT/f"{name}_{mode}.ii").write_text(source)
            source = source.replace(str(PARENT), "solver.cpp").replace(str(CHILD), "solver.cpp")
            source = re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)
            expanded.append(source)
        diff = "".join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True),
                                           fromfile="v028", tofile="v033"))
        (OUT/f"{mode}_preprocessed.diff").write_text(diff)
        record[mode] = {"diff_lines": len(diff.splitlines()), "hunks": diff.count("\n@@")}
    (OUT/"static_verification.json").write_text(json.dumps(record, indent=2)+"\n")
    print("Prepared frozen sources and complete macro-expansion diffs", flush=True)


def build(kind):
    if kind == "production":
        local = ROOT/"target/release/v033_capacity_lns"
        shutil.copy2(local, OUT/"v033_local")
        run(["./scripts/build_solver.sh", "--no-local", "v033_capacity_lns"], "build_production.log")
        shutil.copy2(local, OUT/"v033_production")
    elif kind in ("parent", "child"):
        sanitize = ["-fsanitize=undefined", "-fsanitize-undefined-trap-on-error"] if kind == "child" else []
        run(["g++-15", *FLAGS, "-DLOCAL", *sanitize, "-x", "c++", str(OUT/f"{kind}_fixed_clock.cpp.txt"),
             "-o", str(OUT/f"{kind}_fixed_clock")], f"build_{kind}_fixed_clock.log")
    else:
        raise ValueError(kind)
    print("Built", kind, flush=True)


if __name__ == "__main__":
    import sys
    if len(sys.argv) == 1:
        prepare()
    else:
        build(sys.argv[1])
