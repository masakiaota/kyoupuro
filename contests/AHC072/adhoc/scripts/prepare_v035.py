#!/usr/bin/env python3
"""Prepare diagnostic copies; never change an existing solver or saved result."""
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from make_v035 import transform

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v035_tower_capacity"
PARENT = ROOT / "src/bin/v028_two_order_lns.cpp"
CHILD = ROOT / "src/bin/v035_tower_capacity_lns.cpp"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
CLOCK = "return chrono::duration<double>(chrono::steady_clock::now()-start).count();"


def once(source, old, new):
    if source.count(old) != 1:
        raise RuntimeError(f"Expected one occurrence: {old[:80]}")
    return source.replace(old, new, 1)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    if (OUT / "frozen.json").exists():
        raise RuntimeError("Already frozen")
    parent, child = PARENT.read_text(), CHILD.read_text()
    assert child == transform(parent)
    assert "geo->" not in child and "struct Solver" not in child
    for name, source in (("parent", parent), ("child", child)):
        exposed = source[:source.index("int main() {")]
        exposed = once(exposed, "class TemporalLNS {", "class TemporalLNS {\npublic:")
        exposed = once(exposed, CLOCK, "return 0.0;")
        (OUT / f"{name}_exposed.cpp.txt").write_text(exposed)
    fixed = once(child, "struct Clock {", "struct Clock {\n    mutable uint64_t ticks = 0;")
    fixed = once(fixed, CLOCK, "return (++ticks) * 0.000100003;")
    fixed = once(fixed, "    for(auto m:best)cout", '''    cerr << "[fixed.rng] " << rng.x << "\\n[fixed.ticks] " << clk.ticks << '\\n';
    for(auto m:best)cout''')
    (OUT / "child_fixed_clock.cpp.txt").write_text(fixed)

    old_manifest = json.loads((ROOT / "adhoc/v033_capacity/static_verification.json").read_text())
    originals = [PARENT, ROOT / "src/bin/v000_template.cpp", ROOT / "src/bin/v033_capacity_lns.cpp"]
    originals += sorted((ROOT / "tools/in").glob("*.txt"))
    originals += sorted((ROOT / "results/out/v028_two_order_lns").glob("*.txt"))
    for path in originals:
        assert digest(path) == old_manifest["sources"][str(path.relative_to(ROOT))], path
    for source, destination in ((ROOT / "tools/in", OUT / "fixture/tools/in"),
                                (ROOT / "results/out/v028_two_order_lns", OUT / "fixture/results/out/v028_two_order_lns"),
                                (ROOT / "adhoc/v033_capacity/fixed_clock/parent", OUT / "fixed_clock/parent")):
        destination.mkdir(parents=True, exist_ok=True)
        for path in sorted(source.iterdir()):
            if path.is_file():
                shutil.copy2(path, destination / path.name)
    shutil.copy2(ROOT / "adhoc/v034_capacity_factors/previous_checksums.csv", OUT / "previous_checksums.csv")

    bench = (ROOT / "adhoc/bin/bench_v034_capacity_factors.cpp").read_text()
    bench = bench.replace("bench_v034_capacity_factors.cpp", "bench_v035_tower_capacity.cpp")
    bench = bench.replace("../v034_capacity_factors/", "../v035_tower_capacity/")
    bench = once(bench, "struct Candidate {", "struct BenchInput { int N=0,K=0,cell_count=0; vector<string> C; };\nstruct Candidate {")
    bench = bench.replace("child::Input", "BenchInput")
    begin = bench.index("template<int capacity, int tower_capacity = capacity>\nstruct Compact")
    end = bench.index("static uint64_t cpu_ns()", begin)
    bench = bench[:begin] + '''template<size_t capacity>
struct Compact {
    using Board = child::Board<capacity>;
    using IdentityBoard = child::IdentityBoard;
    using LNS = child::TemporalLNS;
    using Move = child::Move;
    explicit Compact(const BenchInput& in) {
        child::geo.~Geometry();new(&child::geo)child::Geometry{};
        ostringstream text;text<<in.N<<' '<<in.K<<'\\n';
        for(const auto& row:in.C)text<<row<<'\\n';
        istringstream source(text.str());auto* prior=cin.rdbuf(source.rdbuf());cin.clear();
        child::geo.read();cin.rdbuf(prior);cin.clear();child::rng=child::RNG{};
    }
    static auto& geometry(){return child::geo;}
    static uint64_t& random_state(){return child::rng.x;}
    static double random_unit(){return child::rng.unit();}
    static auto smooth(const vector<Move>& moves){return child::smoothRoutes<capacity>(moves);}
};

''' + bench[end:]
    bench = bench.replace("template<int capacity, int tower_capacity = capacity>", "template<size_t capacity>")
    bench = bench.replace("Compact<capacity, tower_capacity>", "Compact<capacity>")
    bench = once(bench, "b.id[i*a.N+j]", "b.id[i][j]")
    bench = once(bench, "new_board=b.initial;", "new_board=b.template initial_board<capacity>();")
    bench = bench[:bench.index("static constexpr array<const char*,5> variant_names")]
    bench += (ROOT / "adhoc/scripts/v035_bench_main.cpp.txt").read_text()
    (ROOT / "adhoc/bin/bench_v035_tower_capacity.cpp").write_text(bench)

    env = os.environ.copy()
    env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
    env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    audit = {"originals": {str(p.relative_to(ROOT)): digest(p) for p in originals},
             "compiler": subprocess.check_output(["g++-15", "--version"], text=True).splitlines()[0],
             "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
             "flags": FLAGS, "sdk": env["SDKROOT"], "deployment_target": "15.0",
             "source_transform_verified": True, "direct_geometry": True}
    for mode, defines in (("local", ["-DLOCAL"]), ("production", ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"])):
        expanded = []
        for name, path in (("parent", PARENT), ("child", CHILD)):
            source = subprocess.check_output(["g++-15", *FLAGS, *defines, "-E", "-P", str(path)], env=env, text=True)
            (OUT / f"{name}_{mode}.ii").write_text(source)
            source = source.replace(str(PARENT), "solver.cpp").replace(str(CHILD), "solver.cpp")
            source = re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)
            expanded.append(source)
        diff = "".join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True), fromfile="v028", tofile="v035"))
        (OUT / f"{mode}_preprocessed.diff").write_text(diff)
        audit[mode] = {"hunks": diff.count("\n@@"), "diff_lines": len(diff.splitlines())}
    (OUT / "static_verification.json").write_text(json.dumps(audit, indent=2) + "\n")
    shutil.copy2(ROOT / "notes/experiments/v035.md", OUT / "preregistration.md")
    print("Prepared diagnostics, frozen inputs, parent results, and fully expanded preprocessing diffs")


if __name__ == "__main__":
    main()
