#!/usr/bin/env python3
"""v205の最小差分を作成し、既存のビルド・前処理検査を使って固定する。"""
import json
from pathlib import Path

import build_v202_v203 as build

ROOT = Path(__file__).resolve().parents[2]
PARENT = "v204_initial_race_tuned"
CHILD = "v205_wide_initial_race"
AUDIT = ROOT / "adhoc/v205_audit"


def main():
    parent = ROOT / f"src/bin/{PARENT}.cpp"
    child = ROOT / f"src/bin/{CHILD}.cpp"
    assert build.sha(parent) == "5d4ed7aa921a9468d5ff290e61bdce84667f9c5b989a624afaf6caece12e89d4"
    assert not child.exists()
    text = parent.read_text()
    changes = []
    edits = [
        (f"// {PARENT}.cpp", f"// {CHILD}.cpp"),
        ("entries.size()==5", "entries.size()==8"),
        ("entries.size()>5", "entries.size()>8"),
        ("""        // 最初の2段階に各15%を使い、最短候補の継続探索へ70%を残す。
        for(int round=0;round<2&&active.size()>1;round++) {
            const double round_end=start+budget*(0.15*(round+1));""",
         """        // 最大8本を半数ずつ育て、2本を比べる段階を挟んでから勝者へ残り65%を渡す。
        // 段階の期限は残余時間の割合とし、候補が先に1本になれば選抜を切り上げる。
        constexpr double round_ends[]={0.15,0.25,0.35};
        for(int round=0;round<3&&active.size()>1;round++) {
            const double round_end=start+budget*round_ends[round];"""),
        ("active.resize(round==0?(active.size()+1)/2:1);", "active.resize((active.size()+1)/2);"),
        ("grow(winner,end,2);", "grow(winner,end,3);"),
    ]
    for old, new in edits:
        assert text.count(old) == 1, old
        offset = text.index(old)
        changes.append(dict(old=old, new=new, offset=offset))
        text = text[:offset]+new+text[offset+len(old):]
    child.write_text(text)
    (AUDIT / f"{CHILD}_changes.json").write_text(json.dumps(changes, ensure_ascii=False, indent=2)+"\n")
    build.PAIRS = [(PARENT, CHILD)]
    build.AUDIT = AUDIT
    build.PARENT_RUNS = {PARENT: "20261003T031601+0900_v204_initial_race_tuned_47557d"}
    build.main()


if __name__ == "__main__":
    main()
