#!/usr/bin/env python3
"""添付v211を登録し、両モードを原版と照合する。solverは実行しない。"""
import argparse
import difflib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

import build_v202_v203 as build
from build_v202_v203 import save, sha
from tune_v204 import evaluator

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v211_audit"
PARENT = "v210_collection_color_quotient"
CHILD = "v211_jump_steiner_collection"
PARENT_RUN = "20261003T113327+0900_v210_collection_color_quotient_5ad8ef"
ATTACHMENT = Path("/Users/masaki/Downloads/v211_jump_steiner_collection.cpp")


def prepare():
    note = ROOT / "notes/experiments/v211.md"
    target = ROOT / f"src/bin/{CHILD}.cpp"
    assert note.exists() and not target.exists() and not AUDIT.exists()
    parent_path = ROOT / f"src/bin/{PARENT}.cpp"
    assert sha(parent_path) == "4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed"
    assert sha(ATTACHMENT) == "fd77f005445d0ea191d9f7f743dfbbbd992fb1f2068dac9ed9d674b11d943889"
    AUDIT.mkdir()
    original = AUDIT / "original.cpp"
    original.write_bytes(ATTACHMENT.read_bytes())
    raw = original.read_text()
    marker = "// 探索の実数演算"
    source = """// v211_jump_steiner_collection.cpp
// v211: 同色3〜6塔の集荷で、ジャンプ経路と合流先を部分集合DPにより共同で選ぶ。
// 直接の親はv210。集荷候補選別、2塔構築、LNS、NN、時間設定を保持する。
// 添付原版の処理と数値を両モードで保持し、この先頭コメントだけ整理した。
// 添付元SHA-256: fd77f005445d0ea191d9f7f743dfbbbd992fb1f2068dac9ed9d674b11d943889
""" + raw[raw.index(marker):]
    parent = parent_path.read_text()

    def block(text, start, end):
        return text[text.index(start):text.index(end, text.index(start))]

    assert block(source, marker, "struct MergePlan") == block(parent, marker, "struct MergePlan")
    assert block(source, "class TemporalLNS", "int main()") == block(parent, "class TemporalLNS", "int main()")
    assert block(source, "    void add_collection(", "    MergePlan plan_collection") == block(
        parent, "    void add_collection(", "    MergePlan plan_collection")
    old_tree = block(source, "    MergePlan plan_collection_tree(", "    MergePlan plan_collection(")
    assert old_tree.replace("plan_collection_tree(", "plan_collection(", 1) == block(
        parent, "    MergePlan plan_collection(", "    void sync(")
    main = source[source.index("int main()"):]
    new_log = block(main, '    cerr<<"jump_collection_calls=', '    cerr<<"baseline=')
    assert main.replace(new_log, "", 1) == parent[parent.index("int main()"):]
    target.write_text(source)
    save(AUDIT / "static_checks.json", dict(clocks_match_v210=True,
         candidate_nomination_matches_v210=True, tree_planner_matches_v210=True,
         temporal_lns_and_selection_match_v210=True, main_changes_only_jump_log=True))
    save(AUDIT / "port_changes.json", [dict(old=raw[:raw.index(marker)],
         new=source[:source.index(marker)], offset=0)])
    old_lines, new_lines = parent.splitlines(True), source.splitlines(True)
    patch, current, offset = [], parent, 0
    for tag, i, j, k, l in difflib.SequenceMatcher(None, old_lines, new_lines, autojunk=False).get_opcodes():
        if tag == "equal":
            continue
        before, after = "".join(old_lines[i:j]), "".join(new_lines[k:l])
        at = len("".join(old_lines[:i])) + offset
        assert current[at:at + len(before)] == before
        patch.append(dict(old=before, new=after, offset=at))
        current = current[:at] + after + current[at + len(before):]
        offset += len(after) - len(before)
    assert current == source
    save(AUDIT / f"{CHILD}_changes.json", patch)
    (AUDIT / "parent.diff").write_text("".join(difflib.unified_diff(
        old_lines, new_lines, fromfile=PARENT, tofile=CHILD)))
    (AUDIT / "preregistration.md").write_bytes(note.read_bytes())
    print(f"Created {CHILD}; parent clocks unchanged; no solver executed.", flush=True)


def compile_and_compare():
    args = argparse.Namespace(bin_name=CHILD, jobs=2, wait_lock=True)
    with evaluator.acquire_eval_lock(args, "v211_build"):
        build.AUDIT = AUDIT
        build.PAIRS = [(PARENT, CHILD)]
        build.PARENT_RUNS = {PARENT: PARENT_RUN}
        build.main()
        env = os.environ.copy()
        if sys.platform == "darwin":
            env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
            env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
        original = AUDIT / "original.cpp"
        child = ROOT / f"src/bin/{CHILD}.cpp"
        checks = {}
        for mode, flags in build.MODES.items():
            before = subprocess.check_output([env.get("CXX", "g++-15"), *build.FLAGS,
                *flags, "-E", "-P", str(original)], env=env, text=True)
            for path in (original, child):
                before = before.replace(str(path), "solver.cpp").replace(f'"{path.name}"', '"solver.cpp"')
            before = re.sub(r'("solver.cpp",\s*)\d+', r'\g<1>0', before)
            after = (AUDIT / f"{CHILD}_{mode}.ii").read_text()
            assert before == after, f"{mode}: unexpected port difference"
            (AUDIT / f"original_{mode}.ii").write_text(before)
            checks[mode] = dict(equal_after_diagnostic_normalization=True,
                                original_preprocessed_sha256=sha(AUDIT / f"original_{mode}.ii"))
        save(AUDIT / "port_preprocessed.json", checks)
        print("Full preprocessing matches attachment in LOCAL and production.", flush=True)


if __name__ == "__main__":
    {"prepare": prepare, "build": compile_and_compare}[sys.argv[1]]()
