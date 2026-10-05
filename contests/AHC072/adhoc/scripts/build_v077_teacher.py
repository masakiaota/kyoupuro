#!/usr/bin/env python3
"""凍結したv076へ採取専用フックを加える。solverの選択規則はここで変更しない。"""
from pathlib import Path
import hashlib

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src/bin/v076_relative_tuned.cpp"
assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == "88b6eadd6d328bf980b53afc96b17320cff1df15f3717cb457ab7df91e3316f4"
s = SOURCE.read_text()

def replace(old, new):
    global s
    assert s.count(old) == 1, (old[:100], s.count(old))
    s = s.replace(old, new)

replace("// v076_relative_tuned.cpp", "// collect_v077_teacher.cpp")
replace("#include <bits/stdc++.h>", "#include <bits/stdc++.h>\n#include <sys/wait.h>\n#include <unistd.h>\n#include <cerrno>\n#ifndef LOCAL\n#error This collector requires LOCAL independent validation.\n#endif")
support = (ROOT / "adhoc/scripts/v077_teacher_support.cpp.txt").read_text()
global_code, member_code = support.split("// NN_MEMBER_CODE\n")
replace("class TemporalLNS {", global_code + "\nclass TemporalLNS {")
replace("    void optimize(vector<Move>& best) {", member_code + "\n    void optimize(vector<Move>& best) {")
replace("        double temperature=lns_start_temperature;\n        while", "        double temperature=lns_start_temperature;\n        try {\n        while")
replace("            time_keeper.check();++iteration;++stagnant;", "            time_keeper.check();++iteration;++stagnant;\n            NnFirstTrialGuard nn_first_guard(current,best);")
replace("                Removal cand=candidates[choice];last_tried[cand.hash]=iteration;", "                if(!nn_child && nn_enabled && !dependency && mode>=3)\n                    nn_snapshot(current,best,stagnant,end,cooldown,choice);\n                Removal cand=candidates[choice];last_tried[cand.hash]=iteration;")
replace("                if(!extracted) {", "                if(!extracted) {\n                    nn_outcome(\"extract_failed\");")
replace("            if(!ok)continue;\n            LOCAL_NOTE(if(repair_priority_broken>=0)", "            if(!ok){nn_outcome(\"insert_failed\");continue;}\n            LOCAL_NOTE(if(repair_priority_broken>=0)")
replace("            if(same_moves(trial,current)){++duplicates;continue;}", "            if(same_moves(trial,current)){nn_outcome(\"duplicate\");++duplicates;continue;}")
replace("            if(same_moves(polished,current)){++duplicates;continue;}", "            if(same_moves(polished,current)){nn_outcome(\"duplicate\");++duplicates;continue;}")
replace("            bool accept=delta<=0", "            // 期限後に完成した候補を教師へ混ぜない。親の受理処理は変更しない。\n            if(nn_child)time_keeper.check();\n            bool accept=delta<=0")
replace("            if(accept) {\n                LOCAL_NOTE(", "            nn_outcome(accept?\"accepted\":\"rejected\");\n            if(nn_child && accept){\n                double accepted_at=time_keeper.exact_elapsed_sec();\n                if(accepted_at>nn_deadline)throw Deadline();\n                nn_last_accept=accepted_at;\n            }\n            if(accept) {\n                LOCAL_NOTE(")
replace("                current=move(polished);rebuild();\n            }\n        }\n    }\n\n};", "                current=move(polished);rebuild();\n            }\n        }\n        } catch(const Deadline&) {\n            if(nn_child)nn_finish(current,best,true);\n            throw;\n        } catch(...) {\n            if(nn_child){cerr<<\"teacher child exception\\n\";_exit(72);}\n            throw;\n        }\n        if(nn_child)nn_finish(current,best,false);\n    }\n\n};")
replace("    for(auto m:best)cout<<board_info.row[m.p]", "    if(nn_enabled) {\n        ofstream f(nn_directory+\"/parent.json\");\n        f<<\"{\\\"snapshots\\\":\"<<nn_next_phase<<\",\\\"pool_free\\\":\"<<state_pool.free_count\n         <<\",\\\"errors\\\":\"<<nn_error_count()<<\",\\\"best_T\\\":\"<<best.size()<<\"}\\n\";\n        f.close();if(!f)_exit(73);\n    }\n    for(auto m:best)cout<<board_info.row[m.p]")
out = ROOT / "adhoc/bin/collect_v077_teacher.cpp"
out.write_text(s)
print(out)
