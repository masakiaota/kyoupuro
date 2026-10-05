#!/usr/bin/env python3
"""v206の登録差分を作り、両モードのビルドと前処理を照合する。solverは実行しない。"""
import difflib
import json
from pathlib import Path

import build_v202_v203 as build

ROOT = Path(__file__).resolve().parents[2]
PARENT = "v204_initial_race_tuned"
CHILD = "v206_pruned_reductions"
AUDIT = ROOT / "adhoc/v206_audit"
PARENT_RUN = "20261003T031601+0900_v204_initial_race_tuned_47557d"


def main():
    parent = ROOT / f"src/bin/{PARENT}.cpp"
    child = ROOT / f"src/bin/{CHILD}.cpp"
    assert build.sha(parent) == "5d4ed7aa921a9468d5ff290e61bdce84667f9c5b989a624afaf6caece12e89d4"
    assert not child.exists()
    AUDIT.mkdir(parents=True, exist_ok=True)
    original = parent.read_text()
    text = original
    changes = []

    def replace(old, new):
        nonlocal text
        assert text.count(old) == 1, old[:160]
        offset = text.index(old)
        changes.append(dict(old=old, new=new, offset=offset))
        text = text[:offset] + new + text[offset+len(old):]

    def remove(begin, after):
        start = text.index(begin)
        end = text.index(after, start)
        replace(text[start:end], "")

    replace(f"// {PARENT}.cpp", f"// {CHILD}.cpp")
    remove("// 1回のBFSで全着地点を調べる。帰巣が起きた着地で輸送を切り、変化後の盤面から再探索する。\nclass FiniteRouter", "enum FinalReduction")
    replace("FINAL_SLACK,FINAL_FINITE,FINAL_KINDS", "FINAL_SLACK,FINAL_KINDS")
    replace('"strict","slack","finite"', '"strict","slack"')
    remove("struct FinalReductionStats {", "struct SearchFocus {")
    replace("    FiniteStats finite;\n", "")
    replace("    static constexpr array<int,4> heavy_kinds={FINAL_JOINT,FINAL_STRICT,FINAL_SLACK,FINAL_FINITE};",
            "    static constexpr array<int,3> heavy_kinds={FINAL_JOINT,FINAL_STRICT,FINAL_SLACK};")
    replace("        // LNS経過時間の一定割合を予算とし、開始直後にも少量の余裕を与える。\n        return 0.03*max(PROGRAM_TIME_LIMIT_SEC*0.02,exact_elapsed_sec)-spent;",
            "        // v204では有限区間が重い短縮の時間の約2/3を使った。残す処理の予算を1%にし、\n        // 削除した分を再挿入へ戻す。開始直後の少量の余裕は親と同じ式で与える。\n        return 0.01*max(PROGRAM_TIME_LIMIT_SEC*0.02,exact_elapsed_sec)-spent;")
    replace("            case FINAL_FINITE:\n                apply(answer,kind,[&]{return compress_finite_windows(answer,deadline,finite,focus.first,focus.last);});break;\n", "")
    replace('            count("finite_attempts",finite.attempts);count("finite_deadlines",finite.deadlines);\n            count("finite_global_deadlines",finite.global_deadlines);count("finite_caps",finite.capped);\n', "")
    replace("    const double start=time_keeper.exact_elapsed_sec(),end=PROGRAM_TIME_LIMIT_SEC;",
            '    // 有限区間の最終仕上げを削り、LOCALで約20msを主探索へ渡す。\n    // 既存の全体期限までは約4msを残し、転送圧縮と4マス共同短縮を行う。\n    const double start=time_keeper.exact_elapsed_sec(),end=PROGRAM_TIME_LIMIT_SEC*1.013;\n    LOCAL_ONLY(trace.add_time_ms("lns_end_limit",end*1000.0));')
    replace('    [[maybe_unused]] const size_t pre_final_reductions_ops=best.size();\n    FinalReductionStats final_reduction_stats;\n    LOCAL_TIME(trace,"final_reductions",[&]{polish_final_reductions(best,final_reduction_stats);});\n    final_reduction_stats.summary();\n    LOCAL_NOTE(trace.count_by("pre_final_reductions_ops",pre_final_reductions_ops);)\n', "")
    replace("        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();",
            "        local_mono_dispatch.summary();coupled_routes.summary();\n        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();")
    for token in ("Finite", "finite_attempts", "FINAL_FINITE", "polish_final_reductions", "FinalReductionStats", "common_tower_prefix"):
        assert token not in text, token
    child.write_text(text)
    build.save(AUDIT / f"{CHILD}_changes.json", changes)
    (AUDIT / "source.diff").write_text("".join(difflib.unified_diff(
        original.splitlines(True), text.splitlines(True), fromfile=PARENT, tofile=CHILD)))
    build.PAIRS = [(PARENT, CHILD)]
    build.AUDIT = AUDIT
    build.PARENT_RUNS = {PARENT: PARENT_RUN}
    build.main()


if __name__ == "__main__":
    main()
