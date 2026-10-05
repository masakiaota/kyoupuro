#!/usr/bin/env python3
"""Verify the one-attribute change without executing a solver."""
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v036_inline_color"
PARENT = ROOT / "src/bin/v035_tower_capacity_lns.cpp"
CHILD = ROOT / "src/bin/v036_inline_color_lns.cpp"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
MODES = {"local": ["-DLOCAL"], "production": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}


def unchanged():
    for name, expected in json.loads((OUT / "source_hashes.json").read_text()).items():
        assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == expected, name


def preprocessing():
    unchanged()
    env = os.environ.copy()
    env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
    env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    result = {"solver_executions": 0, "flags": FLAGS, "modes": {}}
    for mode, defines in MODES.items():
        expanded = []
        for label, path in (("parent", PARENT), ("child", CHILD)):
            run = subprocess.run(["g++-15", *FLAGS, *defines, "-E", "-P", str(path)],
                                 env=env, text=True, capture_output=True, check=True)
            assert not run.stderr, run.stderr
            text = run.stdout
            (OUT / f"{label}_{mode}.ii").write_text(text)
            for source in (PARENT, CHILD):
                text = text.replace(str(source), "solver.cpp")
                text = text.replace('"' + source.name + '"', '"solver.cpp"')
            text = re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', text)
            expanded.append(text)
        diff = "".join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True),
                                           fromfile="v035", tofile="v036"))
        (OUT / f"{mode}_preprocessed.diff").write_text(diff)
        assert expanded[1].count("[[gnu::always_inline]] inline int monoColor") == 1
        assert expanded[0] == expanded[1].replace("[[gnu::always_inline]] inline int monoColor", "inline int monoColor", 1)
        result["modes"][mode] = {"only_mono_color_attribute_changed": True}
    result["compiler"] = subprocess.check_output(["g++-15", "--version"], text=True).splitlines()[0]
    result["cpu"] = subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()
    (OUT / "preprocessing_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print("Both fully expanded sources differ only by the monoColor attribute (diagnostic paths normalized)")


def inspect(binary, mode, label):
    raw = subprocess.check_output(["xcrun", "llvm-nm", "--numeric-sort", "--demangle", str(binary)], text=True)
    (OUT / f"{label}_{mode}_symbols.txt").write_text(raw)
    symbols = {}
    for line in raw.splitlines():
        match = re.match(r"([0-9a-fA-F]+)\s+[Tt]\s+(.*)", line)
        if match:
            symbols[int(match[1], 16)] = match[2]
    color_symbols = [n for n in symbols.values() if "monoColor(" in n]
    entries = {}
    for capacity in (*range(32, 385, 32), 400):
        prefix = f"bool TemporalLNS::insertAt<{capacity}ul>("
        functions = [(a, n) for a, n in symbols.items() if n.startswith(prefix) and "lambda" not in n]
        assert len(functions) == 1, (capacity, functions)
        start, _ = functions[0]
        stop = min(a for a in symbols if a > start)
        asm = subprocess.check_output(["xcrun", "llvm-objdump", "--disassemble", "--no-show-raw-insn",
                                      f"--start-address={start}", f"--stop-address={stop}", str(binary)], text=True)
        calls = []
        for line in asm.splitlines():
            match = re.search(r"\b(?:bl|b)\s+0x([0-9a-fA-F]+)", line)
            if match:
                calls.append(symbols.get(int(match[1], 16), ""))
        entries[str(capacity)] = {"mono_color_transfers": sum("monoColor(" in n for n in calls),
                                  "clock_check_transfers": sum("Clock::check()" in n for n in calls),
                                  "text_bytes_to_next_symbol": stop - start}
        if capacity == 192:
            (OUT / f"{label}_{mode}_insert192.txt").write_text(asm)
    return {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "mono_color_symbols": color_symbols,
            "insert_at": entries}


def assembly():
    unchanged()
    result = {"solver_executions": 0, "modes": {}}
    for mode in MODES:
        parent = inspect(ROOT / "adhoc/v035_tower_capacity" / f"{mode}_solver", mode, "parent")
        child = inspect(OUT / f"{mode}_solver", mode, "child")
        assert not child["mono_color_symbols"], child["mono_color_symbols"]
        assert all(v["mono_color_transfers"] == 1 for v in parent["insert_at"].values())
        assert all(v["mono_color_transfers"] == 0 for v in child["insert_at"].values())
        result["modes"][mode] = {"parent": parent, "child": child, "all_13_capacities_removed": True}
        print(f"{mode}: monoColor transfers 1 -> 0 in all 13 insertAt capacities; no standalone monoColor symbol")
    (OUT / "assembly_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    unchanged()


if __name__ == "__main__":
    {"preprocess": preprocessing, "assembly": assembly}[sys.argv[1]]()
