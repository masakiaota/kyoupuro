#!/usr/bin/env python3
"""Prepare a frozen five-condition capacity comparison without editing solvers."""
import csv
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v034_capacity_factors"


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise RuntimeError(f"Expected one occurrence: {old[:100]!r}")
    return text.replace(old, new, 1)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    OUT.mkdir(exist_ok=True)
    if (OUT / "manifest.json").exists():
        raise RuntimeError("Sources are already frozen")
    old_manifest = json.loads((ROOT / "adhoc/v033_capacity/static_verification.json").read_text())
    originals = [ROOT / "src/bin" / name for name in
                 ("v000_template.cpp", "v028_two_order_lns.cpp", "v033_capacity_lns.cpp")]
    originals += sorted((ROOT / "tools/in").glob("*.txt"))
    originals += sorted((ROOT / "results/out/v028_two_order_lns").glob("*.txt"))
    for path in originals:
        if digest(path) != old_manifest["sources"][str(path.relative_to(ROOT))]:
            raise RuntimeError(f"Changed since v033: {path}")
    for src, dest in ((ROOT / "tools/in", OUT / "fixture/tools/in"),
                      (ROOT / "results/out/v028_two_order_lns", OUT / "fixture/results/out/v028_two_order_lns")):
        dest.mkdir(parents=True, exist_ok=True)
        for path in sorted(src.glob("*.txt")):
            shutil.copy2(path, dest / path.name)
    clock = "return chrono::duration<double>(chrono::steady_clock::now()-start).count();"
    parent = (ROOT / "src/bin/v028_two_order_lns.cpp").read_text()
    child = (ROOT / "src/bin/v033_capacity_lns.cpp").read_text()
    for name, source in (("parent", parent), ("child", child)):
        exposed = source[:source.index("int main() {")]
        exposed = replace_once(exposed, "class TemporalLNS {", "class TemporalLNS {\npublic:")
        exposed = replace_once(exposed, clock, "return 0.0;")
        if name == "child":
            exposed = replace_once(exposed, "template<int capacity>\nstruct Solver {",
                                   "template<int capacity, int tower_capacity = capacity>\nstruct Solver {")
            exposed = replace_once(exposed, "using Board = array<Word, MV>;",
                                   "using Board = array<Word, tower_capacity>;")
        (OUT / f"{name}_exposed.cpp.txt").write_text(exposed)

    bench = (ROOT / "adhoc/bin/bench_v033_capacity.cpp").read_text()
    bench = bench.replace("bench_v033_capacity.cpp", "bench_v034_capacity_factors.cpp")
    bench = bench.replace("../v033_capacity/", "../v034_capacity_factors/")
    bench = replace_once(bench, "template<int capacity>\nstruct Compact {",
                         "template<int capacity, int tower_capacity = capacity>\nstruct Compact {")
    bench = replace_once(bench, "using Engine = child::Solver<capacity>;",
                         "using Engine = child::Solver<capacity, tower_capacity>;")
    bench = replace_once(bench, "template<int capacity>\nstatic size_t replay_case",
                         "template<int capacity, int tower_capacity = capacity>\nstatic size_t replay_case")
    bench = bench.replace("Compact<capacity> compact", "Compact<capacity, tower_capacity> compact")
    bench = bench.replace("typename Compact<capacity>::", "typename Compact<capacity, tower_capacity>::")
    bench = replace_once(bench, "    array<uint64_t,3> ns{};",
                         "    array<uint64_t,3> ns{}, wall_ns{};")
    bench = replace_once(bench, "    auto started=cpu_ns();",
                         "    auto wall_started=chrono::steady_clock::now();\n    auto started=cpu_ns();")
    bench = replace_once(bench, "    result.ns[0]=cpu_ns()-started;",
                         "    result.ns[0]=cpu_ns()-started;\n    result.wall_ns[0]=chrono::duration_cast<chrono::nanoseconds>(chrono::steady_clock::now()-wall_started).count();")
    bench = replace_once(bench, "\n        started=cpu_ns();\n",
                         "\n        wall_started=chrono::steady_clock::now();started=cpu_ns();\n")
    bench = replace_once(bench, "        result.ns[1]+=cpu_ns()-started;",
                         "        result.ns[1]+=cpu_ns()-started;\n        result.wall_ns[1]+=chrono::duration_cast<chrono::nanoseconds>(chrono::steady_clock::now()-wall_started).count();")
    bench = replace_once(bench, "            started=cpu_ns();polished=A::smooth(rebuilt);result.ns[2]+=cpu_ns()-started;",
                         "            wall_started=chrono::steady_clock::now();started=cpu_ns();polished=A::smooth(rebuilt);result.ns[2]+=cpu_ns()-started;\n            result.wall_ns[2]+=chrono::duration_cast<chrono::nanoseconds>(chrono::steady_clock::now()-wall_started).count();")
    bench = bench[:bench.index("int main(int argc,char** argv) {")] + (ROOT / "adhoc/scripts/v034_bench_main.cpp.txt").read_text()
    target = ROOT / "adhoc/bin/bench_v034_capacity_factors.cpp"
    target.write_text(bench)
    old_rows = list(csv.DictReader((ROOT / "adhoc/v033_capacity/fixed_work_samples.csv").open()))
    old_checksums = {}
    for row in old_rows:
        value = (row["candidates"], row["completed"], row["checksum"])
        if row["case"] in old_checksums and value != old_checksums[row["case"]]:
            raise RuntimeError("Inconsistent previous workload")
        old_checksums[row["case"]] = value
    with (OUT / "previous_checksums.csv").open("w") as stream:
        stream.write("case,candidates,completed,checksum\n")
        for case, values in sorted(old_checksums.items()):
            stream.write(case + "," + ",".join(values) + "\n")
    shutil.copy2(ROOT / "notes/experiments/v034.md", OUT / "preregistration.md")
    originals += [ROOT / "adhoc/bin/bench_v033_capacity.cpp"]
    frozen = [target, ROOT / "adhoc/scripts/prepare_v034_capacity.py", ROOT / "adhoc/scripts/run_v034_capacity.py",
              ROOT / "adhoc/scripts/v034_bench_main.cpp.txt", ROOT / "notes/experiments/v034.md"]
    frozen += sorted(p for p in OUT.rglob("*") if p.is_file())
    record = {"originals": {str(p.relative_to(ROOT)): digest(p) for p in originals},
              "frozen": {str(p.relative_to(ROOT)): digest(p) for p in frozen},
              "compiler": subprocess.check_output(["g++-15", "--version"], text=True).splitlines()[0],
              "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
              "source_changes": ["Clock returns zero in both diagnostic copies", "TemporalLNS exposed in both copies",
                                 "Separate compile-time Board capacity in child diagnostic copy"],
              "previous_cases": len(old_checksums)}
    (OUT / "manifest.json").write_text(json.dumps(record, indent=2) + "\n")
    print(f"Prepared {target.name}: {len(old_checksums)} frozen cases, five variants")


if __name__ == "__main__":
    main()
