#!/usr/bin/env python3
"""v212の作成と両モードの前処理照合。solverは実行しない。"""
import argparse
import difflib
import json
import os
from pathlib import Path
import subprocess
import sys

import build_v202_v203 as build
from build_v202_v203 import save, sha
from tune_v204 import evaluator

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v212_audit"
PARENT = "v210_collection_color_quotient"
CHILD = "v212_mono_packet_quotient"
PARENT_RUN = "20261003T113327+0900_v210_collection_color_quotient_5ad8ef"


def prepare():
    target = ROOT / f"src/bin/{CHILD}.cpp"
    note = ROOT / "notes/experiments/v212.md"
    assert note.exists() and not target.exists() and not AUDIT.exists()
    parent_path = ROOT / f"src/bin/{PARENT}.cpp"
    assert sha(parent_path) == "4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed"
    original = parent_path.read_text()
    source = original

    def replace(old, new):
        nonlocal source
        assert source.count(old) == 1, old[:100]
        source = source.replace(old, new, 1)

    marker = "// 探索の実数演算"
    source = ("// v212_mono_packet_quotient.cpp\n"
              "// v212: 単色の束の再挿入で、同じ色列になる隙間のラベルを共有する。\n"
              "// v210を親とし、構築・候補選択・1匹再挿入・時間設定を保持する。\n"
              + source[source.index(marker):])
    replace("struct DependencyStats {", """struct PacketAliasStats {
    int64_t mono_calls=0,closures=0,gap_relaxations=0,top_relaxations=0,cost_relaxations=0,completed=0;
    void summary() const {
        auto count=[&](const char* key,int64_t value){trace.count_by(string("packet_alias_")+key,value);};
        count("mono_calls",mono_calls);count("closures",closures);
        count("gap_relaxations",gap_relaxations);count("top_relaxations",top_relaxations);
        count("cost_relaxations",cost_relaxations);count("completed",completed);
    }
} local_packet_alias;
struct DependencyStats {""")
    replace("        if(cap<0||!packet)return false;\n", """        if(cap<0||!packet)return false;
        const int packet_color=mono_color_code(packet);
        LOCAL_NOTE(local_packet_alias.mono_calls+=packet_color>0;)
""")
    replace("        auto ceiling=[&](){return min(cap,labels[GOAL].cost);};\n", """        auto ceiling=[&](){return min(cap,labels[GOAL].cost);};
        // 単色の束は反転しても同じ列で、帰巣時には全体が消えるため語は1種類だけである。
        // 背景の同色連続部分をまたいでも実盤面は一致し、追加操作なしで足場役と乗客役を交換できる。
        auto close_packet_gaps=[&](int p,bool top_only) {
            if(packet_color<=0)return;
            LOCAL_NOTE(++local_packet_alias.closures;)
            const int hp=h[p];
            if(!hp||hp+sz[0]>8)return;
            Label* row=labels.data()+p*8;
            const TowerBits word=b[p];
            if(top_only) {
                if(int((word>>(4*(hp-1)))&15)!=packet_color)return;
                int first=hp-1;
                while(first>0&&int((word>>(4*(first-1)))&15)==packet_color)--first;
                Label best=row[hp];
                for(int g=first;g<hp;g++)if(better(row[g].cost,row[g].bonus,best))best=row[g];
                if(better(best.cost,best.bonus,row[hp])) {
                    LOCAL_NOTE(++local_packet_alias.top_relaxations;
                               local_packet_alias.cost_relaxations+=best.cost<row[hp].cost;)
                    row[hp]=best;
                }
                return;
            }
            for(int first=0;first<hp;) {
                if(int((word>>(4*first))&15)!=packet_color){++first;continue;}
                int last=first+1;
                while(last<hp&&int((word>>(4*last))&15)==packet_color)++last;
                Label best=row[first];
                for(int g=first+1;g<=last;g++)if(better(row[g].cost,row[g].bonus,best))best=row[g];
                for(int g=first;g<=last;g++)if(better(best.cost,best.bonus,row[g])) {
                    LOCAL_NOTE(++local_packet_alias.gap_relaxations;
                               local_packet_alias.cost_relaxations+=best.cost<row[g].cost;)
                    row[g]=best;
                }
                first=last;
            }
        };
""")
    replace("        auto& from_p=packet_from_p;auto& from_q=packet_from_q;\n        seed(source);", """        auto& from_p=packet_from_p;auto& from_q=packet_from_q;
        close_packet_gaps(source,true);
        seed(source);""")
    replace("            for(int i=0;i<W;i++)for(int g=0;g<8;g++) {", """            close_packet_gaps(p,false);close_packet_gaps(q,false);
            for(int i=0;i<W;i++)for(int g=0;g<8;g++) {""")
    replace("            bool reopen_p=h[p]!=hp,reopen_q=h[q]!=hq;\n            for(int i=0;i<W;i++) {", """            close_packet_gaps(p,true);close_packet_gaps(q,true);
            bool reopen_p=h[p]!=hp,reopen_q=h[q]!=hq;
            for(int i=0;i<W;i++) {""")
    replace('            throw logic_error("packet reinsertion cost mismatch");\n        return true;', '''            throw logic_error("packet reinsertion cost mismatch");
        LOCAL_NOTE(local_packet_alias.completed+=packet_color>0;)
        return true;''')
    replace("        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();", "        local_packet_alias.summary();\n        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();")

    AUDIT.mkdir()
    target.write_text(source)
    before_lines, after_lines = original.splitlines(True), source.splitlines(True)
    changes, current, offset = [], original, 0
    for tag, i, j, k, l in difflib.SequenceMatcher(None, before_lines, after_lines, autojunk=False).get_opcodes():
        if tag == "equal":
            continue
        before, after = "".join(before_lines[i:j]), "".join(after_lines[k:l])
        at = len("".join(before_lines[:i])) + offset
        assert current[at:at + len(before)] == before
        changes.append(dict(old=before, new=after, offset=at))
        current = current[:at] + after + current[at + len(before):]
        offset += len(after) - len(before)
    assert current == source
    save(AUDIT / f"{CHILD}_changes.json", changes)
    (AUDIT / "parent.diff").write_text("".join(difflib.unified_diff(before_lines, after_lines,
        fromfile=PARENT, tofile=CHILD)))
    (AUDIT / "preregistration.md").write_bytes(note.read_bytes())
    print(f"Created {CHILD}; solver executions=0", flush=True)


def compile_and_compare():
    args = argparse.Namespace(bin_name=CHILD, jobs=2, wait_lock=True)
    with evaluator.acquire_eval_lock(args, "v212_build"):
        build.AUDIT = AUDIT
        build.PAIRS = [(PARENT, CHILD)]
        build.PARENT_RUNS = {PARENT: PARENT_RUN}
        build.main()
        env = os.environ.copy()
        if sys.platform == "darwin":
            env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
            env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
        command = [env.get("CXX", "g++-15"), *build.FLAGS, "-DLOCAL",
                   str(ROOT / "adhoc/bin/check_v212_packet_quotient.cpp"),
                   "-o", str(AUDIT / "check_packet")]
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True)
        (AUDIT / "probe_build.log").write_text(result.stdout + result.stderr)
        assert result.returncode == 0, result.stderr
        assert not result.stderr, result.stderr
        print("Fixed-problem checker built; solver executions=0", flush=True)


if __name__ == "__main__":
    {"prepare": prepare, "build": compile_and_compare}[sys.argv[1]]()
