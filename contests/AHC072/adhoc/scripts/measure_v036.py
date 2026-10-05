#!/usr/bin/env python3
"""Measure frozen v028/v035/v036 candidate work with their real clocks."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v036_measurement"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp", "-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]
SOURCES = {"parent": "v028_two_order_lns", "v035": "v035_tower_capacity_lns", "v036": "v036_inline_color_lns"}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def unchanged():
    for name, expected in json.loads((OUT / "frozen.json").read_text())["hashes"].items():
        if digest(ROOT / name) != expected:
            raise RuntimeError("Changed during measurement: " + name)


def prepare():
    OUT.mkdir(exist_ok=True)
    assert not (OUT / "frozen.json").exists()
    protected = json.loads((ROOT / "adhoc/v036_inline_color/source_hashes.json").read_text())
    for name, expected in protected.items():
        assert digest(ROOT / name) == expected, name
    for namespace, name in SOURCES.items():
        source = (ROOT / "src/bin" / (name + ".cpp")).read_text()
        assert source.count("class TemporalLNS {") == source.count("int main() {") == 1
        exposed = source.replace("class TemporalLNS {", "class TemporalLNS {\npublic:", 1)
        exposed = exposed.replace("int main() {", "int retained_solver_entry() {", 1)
        # mainだけに許される暗黙のreturn 0を、未実行の保存用入口に明記する。
        if namespace == "parent":
            assert exposed.endswith("}\n")
            exposed = exposed[:-2] + "    return 0;\n}\n"
        restored = exposed.replace("class TemporalLNS {\npublic:", "class TemporalLNS {", 1)
        restored = restored.replace("int retained_solver_entry() {", "int main() {", 1)
        if namespace == "parent":
            restored = restored.removesuffix("    return 0;\n}\n") + "}\n"
        assert restored == source
        assert "return chrono::duration<double>(chrono::steady_clock::now()-start).count();" in exposed
        (OUT / (namespace + "_exposed.cpp.txt")).write_text(exposed)
    old = ROOT / "adhoc/v035_tower_capacity"
    old_manifest = json.loads((old / "frozen.json").read_text())
    for path in sorted((old / "fixture").rglob("*.txt")):
        rel = path.relative_to(old / "fixture")
        assert digest(path) == old_manifest["frozen"][str(path.relative_to(ROOT))]
        assert digest(ROOT / rel) == digest(path)
        dest = OUT / "fixture" / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
        protected[str(rel)] = digest(path)
    shutil.copy2(old / "previous_checksums.csv", OUT / "previous_checksums.csv")
    shutil.copy2(ROOT / "notes/experiments/v036.md", OUT / "preregistration.md")
    (OUT / "protected.json").write_text(json.dumps(protected, indent=2) + "\n")

    previous = (ROOT / "adhoc/bin/bench_v035_tower_capacity.cpp").read_text()
    header = '// bench_v036_real_clock.cpp\n#include <bits/stdc++.h>\n#include <time.h>\n'
    for ns in SOURCES:
        header += f'namespace {ns} {{\n#include "../v036_measurement/{ns}_exposed.cpp.txt"\n}}\n'
    begin = previous.index("using namespace std;")
    end = previous.index("template<size_t capacity>\nstruct Compact")
    adapters = previous[begin:end].replace("parent::rng=parent::RNG{};", "parent::rng=parent::RNG{};\n        parent::clk.start=chrono::steady_clock::now();parent::clk.limit=3600.0;")
    end2 = previous.index("static uint64_t cpu_ns()", end)
    compact = previous[end:end2]
    for ns in ("v035", "v036"):
        one = compact.replace("Compact", ns.upper()).replace("child::", ns + "::")
        one = one.replace(f"{ns}::rng={ns}::RNG{{}};", f"{ns}::rng={ns}::RNG{{}};\n        {ns}::clk.start=chrono::steady_clock::now();{ns}::clk.limit=3600.0;")
        adapters += one
    common = previous[end2:previous.index("template<size_t capacity>\nstatic size_t replay_case")]
    checksum = previous[previous.index("static uint64_t checksum"):previous.index("static constexpr array<const char*,3> variant_names")]
    main = (ROOT / "adhoc/scripts/v036_bench_main.cpp.txt").read_text()
    (ROOT / "adhoc/bin/bench_v036_real_clock.cpp").write_text(header + adapters + common + checksum + main)
    print("Prepared identical work with original steady_clock bodies", flush=True)


def build():
    assert not (OUT / "frozen.json").exists()
    env = os.environ.copy()
    env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
    env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    args = ["g++-15", *FLAGS, str(ROOT / "adhoc/bin/bench_v036_real_clock.cpp"), "-o", str(OUT / "bench")]
    with (OUT / "build.log").open("w") as log:
        subprocess.run(args, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    print("Built nonLOCAL benchmark", flush=True)


def inspect():
    raw = subprocess.check_output(["xcrun", "llvm-nm", "--numeric-sort", "--demangle", str(OUT / "bench")], text=True)
    (OUT / "symbols.txt").write_text(raw)
    symbols = {}
    for line in raw.splitlines():
        m = re.match(r"([0-9a-fA-F]+)\s+[Tt]\s+(.*)", line)
        if m:
            symbols[int(m[1], 16)] = m[2]
    result = {}
    for ns in SOURCES:
        entries = {}
        for capacity in ([400] if ns == "parent" else [*range(32, 385, 32), 400]):
            prefix = "parent::TemporalLNS::insertAt(" if ns == "parent" else f"bool {ns}::TemporalLNS::insertAt<{capacity}ul>("
            functions = [(a, n) for a, n in symbols.items() if n.startswith(prefix) and "lambda" not in n]
            assert len(functions) == 1, (ns, capacity, functions)
            start, _ = functions[0]
            stop = min(a for a in symbols if a > start)
            asm = subprocess.check_output(["xcrun", "llvm-objdump", "--disassemble", "--no-show-raw-insn",
                                           f"--start-address={start}", f"--stop-address={stop}", str(OUT / "bench")], text=True)
            calls = []
            for line in asm.splitlines():
                m = re.search(r"\b(?:bl|b)\s+0x([0-9a-fA-F]+)", line)
                if m:
                    calls.append(symbols.get(int(m[1], 16), ""))
            entries[str(capacity)] = {"mono_color_transfers": sum("monoColor(" in n for n in calls),
                                     "clock_check_transfers": sum("Clock::check()" in n for n in calls),
                                     "text_bytes_to_next_symbol": stop - start}
            if capacity in (192, 400):
                (OUT / f"{ns}_insert{capacity}.txt").write_text(asm)
        result[ns] = entries
    assert not any("v036::monoColor(" in n for n in symbols.values())
    assert all(x["mono_color_transfers"] == 0 for x in result["v036"].values())
    assert all(x["clock_check_transfers"] > 0 for entries in result.values() for x in entries.values())
    (OUT / "assembly_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({ns: entries.get("192", entries["400"]) for ns, entries in result.items()}, indent=2), flush=True)


def freeze():
    assert not (OUT / "frozen.json").exists()
    hashes = json.loads((OUT / "protected.json").read_text())
    paths = list(OUT.rglob("*")) + [ROOT / "adhoc/bin/bench_v036_real_clock.cpp"]
    paths += [ROOT / "adhoc/scripts" / n for n in ("measure_v036.py", "v036_bench_main.cpp.txt", "summarize_v036.py")]
    paths += [ROOT / "adhoc/v036_inline_color" / n for n in ("local_solver", "production_solver", "assembly_summary.json")]
    for name in ("v028_two_order_lns", "v035_tower_capacity_lns"):
        paths += sorted((ROOT / "results/out" / name).glob("*.txt*"))
    for path in paths:
        if path.is_file():
            hashes[str(path.relative_to(ROOT))] = digest(path)
    data = {"hashes": hashes, "flags": FLAGS,
            "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
            "compiler": subprocess.check_output(["g++-15", "--version"], text=True).splitlines()[0]}
    (OUT / "frozen.json").write_text(json.dumps(data, indent=2) + "\n")
    unchanged()
    print(f"Frozen {len(hashes)} files", flush=True)


def measure():
    unchanged()
    assert not (OUT / "fixed_work_samples.csv").exists(), "Already measured"
    with (ROOT / "results/.eval.lock").open("a+") as lock:
        print("Waiting for evaluation lock", flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        with (OUT / "measurement.log").open("w") as log:
            process = subprocess.Popen([str(OUT / "bench"), str(OUT / "fixture"), str(OUT)], cwd=ROOT,
                                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            for line in process.stdout:
                log.write(line)
                log.flush()
                print(line, end="", flush=True)
            if process.wait():
                raise RuntimeError("Benchmark failed; see measurement.log")
    unchanged()
    print("All frozen files unchanged", flush=True)


if __name__ == "__main__":
    {"prepare": prepare, "build": build, "inspect": inspect, "freeze": freeze, "measure": measure}[sys.argv[1]]()
