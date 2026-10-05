#!/usr/bin/env python3
"""v209にv208の登録済み集荷差分を適用する。solverは実行しない。"""
import argparse
import difflib
import json
from pathlib import Path
import sys

import build_v202_v203 as build
from build_v202_v203 import save, sha
from tune_v204 import evaluator

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v210_audit"
PARENT = "v209_color_quotient_reinsertion"
COLLECTION = "v208_mono_collection"
CHILD = "v210_collection_color_quotient"
PARENT_RUN = "20261003T111702+0900_v209_color_quotient_reinsertion_2177b3"


def prepare():
    note = ROOT / "notes/experiments/v210.md"
    target = ROOT / f"src/bin/{CHILD}.cpp"
    assert note.exists() and not target.exists() and not AUDIT.exists()
    parent_path = ROOT / f"src/bin/{PARENT}.cpp"
    collection_path = ROOT / f"src/bin/{COLLECTION}.cpp"
    assert sha(parent_path) == "ce0080eac68ee603bc4fc392e30f648f725a31f13545bdb2a9af1c2a93a50c04"
    assert sha(collection_path) == "de5cf6717e159d3a3fdb0ef783f4799733d77fe652415351f54a801586aa39a1"
    source = parent_path.read_text()
    changes = []

    def replace(old, new):
        nonlocal source
        assert source.count(old) == 1
        at = source.index(old)
        changes.append(dict(old=old, new=new, offset=at))
        source = source[:at] + new + source[at + len(old):]

    header = source[:source.index("// 探索の実数演算")]
    replace(header, """// v210_collection_color_quotient.cpp
// v210: v209の同色再挿入・候補保持と、v208の3塔以上の同色集荷構築を併用する。
// 集荷の計画と採否判定はv208からそのまま継承する。
// LOCAL・非LOCALとも時間管理、初期解選抜、探索と終了処理はv209を継承する。
""")
    inherited = json.loads((ROOT / "adhoc/v208_audit/v208_mono_collection_changes.json").read_text())
    assert len(inherited) == 11
    for change in inherited[1:]:
        replace(change["old"], change["new"])

    def block(text, start, end):
        return text[text.index(start):text.index(end, text.index(start))]

    # 両実装の境界でだけ組み合わせ、内部の手順と数値を保持する。
    collection_start = "// 同色の3塔以上を一度に評価する。"
    assert block(source, collection_start, "class PortionRouter") == block(
        collection_path.read_text(), collection_start, "class PortionRouter")
    assert block(source, "class TemporalLNS", "struct InitialSolutions") == block(
        parent_path.read_text(), "class TemporalLNS", "struct InitialSolutions")
    assert block(source, "// 探索の実数演算", collection_start) == block(
        parent_path.read_text(), "// 探索の実数演算", "class Constructor")
    AUDIT.mkdir()
    target.write_text(source)
    save(AUDIT / f"{CHILD}_changes.json", changes)
    save(AUDIT / "static_checks.json", dict(collection_matches_v208=True,
         temporal_lns_matches_v209=True,clocks_match_v209=True,collection_patches=10))
    (AUDIT / "parent.diff").write_text("".join(difflib.unified_diff(
        parent_path.read_text().splitlines(True), source.splitlines(True), fromfile=PARENT, tofile=CHILD)))
    (AUDIT / "preregistration.md").write_bytes(note.read_bytes())
    print(f"Created {CHILD}; v208 collection and v209 LNS unchanged; no solver executed.", flush=True)


def compile_and_compare():
    args = argparse.Namespace(bin_name=CHILD, jobs=2, wait_lock=True)
    with evaluator.acquire_eval_lock(args, "v210_build"):
        build.AUDIT = AUDIT
        build.PAIRS = [(PARENT, CHILD)]
        build.PARENT_RUNS = {PARENT: PARENT_RUN}
        build.main()


if __name__ == "__main__":
    {"prepare": prepare, "build": compile_and_compare}[sys.argv[1]]()
