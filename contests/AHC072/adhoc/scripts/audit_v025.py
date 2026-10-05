#!/usr/bin/env python3
"""Compile-only audit of fast-math and the template's small tower functions."""
import collections
import difflib
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v025_fast_math"
PARENT = "v022_dependency_lns"
CHILD = "v025_fast_math"
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
MODES = {"local": ["-DLOCAL"], "submission": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}


def run(args, text=None):
    result = subprocess.run(["g++-15", *args], input=text, capture_output=True, text=True, cwd=ROOT)
    if result.returncode:
        raise RuntimeError(result.stderr)
    if result.stderr:
        raise RuntimeError("Unexpected compiler diagnostic: " + result.stderr)
    return result.stdout


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def function_body(assembly, name):
    # This audit targets the recorded macOS arm64 environment.
    match = re.search(rf"(?ms)^_{name}:\n(.*?^LFE\d+:)", assembly)
    if not match:
        raise RuntimeError(f"Missing assembly function: {name}")
    labels = {}
    return re.sub(r"\bL(?:FB|FE|CFI)?\d+\b",
                  lambda m: labels.setdefault(m[0], f"L{len(labels)}"), match[0])


def main():
    OUT.mkdir(exist_ok=True)
    paths = {name: ROOT / f"src/bin/{name}.cpp" for name in (PARENT, CHILD)}
    parent, child = (paths[name].read_text() for name in (PARENT, CHILD))
    normalized = child.replace(CHILD, PARENT).replace(
        '// 探索用の実数計算は丸め差を許容し、fast-mathを有効にする。',
        "// AtCoder's default -O2 is overridden for the solver, without fast-math.",
    ).replace('omit-frame-pointer,fast-math', 'omit-frame-pointer')
    if normalized != parent:
        raise RuntimeError("Unexpected source difference")
    (OUT / "source.diff").write_text("".join(difflib.unified_diff(
        parent.splitlines(True), child.splitlines(True), fromfile=PARENT, tofile=CHILD)))
    report = {"source_sha256": {name: digest(path) for name, path in paths.items()},
              "only_fast_math_and_comments_changed": True, "preprocessing": {}, "code_generation": {}}
    for mode, defines in MODES.items():
        expanded = []
        for name in (PARENT, CHILD):
            output = run([*FLAGS, *defines, "-E", "-P", str(paths[name])])
            expanded.append(output.replace(name, "solver"))
        expected = expanded[1].replace('omit-frame-pointer,fast-math', 'omit-frame-pointer')
        diff = "".join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True),
                                         fromfile=PARENT, tofile=CHILD))
        (OUT / f"preprocessed_{mode}.diff").write_text(diff)
        report["preprocessing"][mode] = {
            "only_pragma_differs_after_filename_normalization": expected == expanded[0],
            "diff_lines": len(diff.splitlines()),
            "expanded_sha256": [hashlib.sha256(s.encode()).hexdigest() for s in expanded],
        }
        if expected != expanded[0]:
            raise RuntimeError(f"Unexpected fully preprocessed difference in {mode}")
    for name in (PARENT, CHILD):
        assembly_path = OUT / f"{name}_local.s"
        tree_path = OUT / f"{name}_local.optimized"
        run([*FLAGS, *MODES["local"], f"-fdump-tree-optimized={tree_path}",
             "-S", str(paths[name]), "-o", str(assembly_path)])
        assembly, tree = assembly_path.read_text(), tree_path.read_text()
        instructions = collections.Counter(re.findall(r"(?m)^\s+(f[a-z0-9]+)\s", assembly))
        calls = collections.Counter(re.findall(r"\bbl\s+_(exp|pow|sqrt)\b", assembly))
        report["code_generation"][name] = {
            "fast_math_attribute_lines": sum("__attribute__" in line and "fast-math" in line
                                             for line in tree.splitlines()),
            "floating_instruction_counts": dict(sorted(instructions.items())),
            "math_library_call_sites": dict(sorted(calls.items())),
        }
    if report["code_generation"][CHILD]["fast_math_attribute_lines"] == 0:
        raise RuntimeError("fast-math attribute not found in optimized IR")

    template_path = ROOT / "src/bin/v000_template.cpp"
    source = template_path.read_text().replace("int main() {", "int v000_template_main() {", 1)
    probe = r'''
extern "C" int probe_tower_read(Tower t, int i) {
    return t.height() + t.at(i) + t.top() + t.bottom() + t.empty();
}
extern "C" void probe_tower_edges(Tower& t, int c) {
    t.push_bottom(c); t.pop_top(); t.push_top(c); t.pop_bottom(); t.reverse();
}
extern "C" int probe_tower_pair(Tower& source, Tower& dest, int k, int sc, int dc) {
    const int h = source.height(), g = dest.height(), n = h - k;
    Tower moving = source.take_top(n, h);
    moving.reverse(n); dest.append_top(moving, g);
    return source.home(sc, k) + dest.home(dc, g + n);
}
'''
    (OUT / "tower_probe.txt").write_text(probe)
    begin, end = source.index("struct Tower {"), source.index("struct State {")
    forced_tower = re.sub(r"(?m)^    ((?:bool|int|void|Tower) \w+\()",
                          r"    [[gnu::always_inline]] \1", source[begin:end])
    versions = {"automatic": source, "forced": source[:begin] + forced_tower + source[end:]}
    bodies = {}
    for variant, code in versions.items():
        assembly = run([*FLAGS, *MODES["local"], "-S", "-x", "c++", "-", "-o", "-"], code + probe)
        bodies[variant] = {name: function_body(assembly, name) for name in (
            "probe_tower_read", "probe_tower_edges", "probe_tower_pair")}
        (OUT / f"tower_{variant}.s").write_text("\n".join(bodies[variant].values()))
    report["template"] = {
        "sha256": digest(template_path),
        "probes_identical_with_forced_inline": bodies["automatic"] == bodies["forced"],
        "automatic_probe_calls": {name: re.findall(r"\bbl\s+(\S+)", body)
                                  for name, body in bodies["automatic"].items()},
        "restrict_scope": "Board constructor's BFS distance row; ray and queue are disjoint",
    }
    report["input_sha256"] = {p.name: digest(p) for p in sorted((ROOT / "tools/in").glob("*.txt"))}
    (OUT / "static_verification.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "input_sha256"},
                     ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
