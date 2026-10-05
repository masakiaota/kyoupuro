#!/usr/bin/env python3
"""凍結v079へ、候補ごとの決定的な長期教師採取だけを加える。"""
from pathlib import Path
import hashlib

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src/bin/v079_nn_immediate.cpp"
assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == "a58af5f053bfbf6ce33463c60e21d0bbf6f659479c6aadb5d452d158da65bb8a"
s = SOURCE.read_text()


def replace(old, new):
    global s
    assert s.count(old) == 1, (old[:100], s.count(old))
    s = s.replace(old, new)


replace("// v079_nn_immediate.cpp", "// collect_v083_teacher.cpp")
replace("#include <bits/stdc++.h>", """#include <bits/stdc++.h>
#include <sys/wait.h>
#include <unistd.h>
#include <signal.h>
#include <cerrno>
#if !defined(LOCAL) || defined(NDEBUG)
#error This collector requires LOCAL and active assertions.
#endif""")
replace("struct TimeKeeper {", """// 子の時計だけを照会回数で進め、機械負荷に依存する枝分かれを除く。
struct Nn083WallLimit {};
bool nn083_clock_active=false;
uint64_t nn083_clock_ticks=0;
double nn083_clock_origin=0;
chrono::steady_clock::time_point nn083_wall_start;
struct TimeKeeper {""")
replace("    double exact_elapsed_sec() const {\n        return", """    double exact_elapsed_sec() const {
        if(nn083_clock_active){
            if(chrono::duration<double>(chrono::steady_clock::now()-nn083_wall_start).count()>10.0)throw Nn083WallLimit();
            return nn083_clock_origin+(++nn083_clock_ticks)*(PROGRAM_TIME_LIMIT_SEC*1e-6);
        }
        return""")
support = (ROOT / "adhoc/scripts/v083_teacher_support.cpp.txt").read_text()
global_code, member_code = support.split("// NN083_MEMBER_CODE\n")
replace("class TemporalLNS {", global_code + "\nclass TemporalLNS {")
replace("    void optimize(vector<Move>& best) {", member_code + "\n    void optimize(vector<Move>& best) {")
replace("        while(time_keeper.exact_elapsed_sec()<end) {\n            time_keeper.check();++iteration;++stagnant;", """        try {
        while(nn083_clock_active?nn083_step<=nn083_steps:time_keeper.exact_elapsed_sec()<end) {
            // 反復ごとに対応する乱数へ戻し、前反復の消費数の差を持ち越さない。
            if(nn083_clock_active){
                rng.x=nn083_seed(nn083_phase,nn083_replica,nn083_step);
                nn083_seed_digest=nn083_mix(nn083_seed_digest,rng.x);
            }
            time_keeper.check();++iteration;++stagnant;
            Nn083TrialGuard nn083_guard(current,best);""")
old = "clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(LOCAL_SECONDS(0.02),end-lns_start),0.0,1.0)"
assert s.count(old) == 2
s = s.replace(old, f"(nn083_clock_active?nn083_progress():{old})")
replace("                temperature=lns_start_temperature*pow(lns_end_temperature/lns_start_temperature,progress);", """                temperature=lns_start_temperature*pow(lns_end_temperature/lns_start_temperature,progress);
                if(nn083_clock_active)nn083_temperature=temperature;""")
replace("                if(candidates.empty())break;", """                if(candidates.empty()){
                    if(nn083_clock_active){nn083_absorbed=true;nn083_stop_reason="no_candidates";}
                    break;
                }""")
replace("                Removal cand=candidates[choice];last_tried[cand.hash]=iteration;", """                if(nn083_enabled&&!nn083_clock_active&&nn_eligible)
                    nn083_snapshot(current,best,stagnant,end,cooldown,choice,temperature);
                Removal cand=candidates[choice];last_tried[cand.hash]=iteration;""")
replace("                if(!extracted) {", "                if(!extracted) {\n                    nn083_outcome(\"extract_failed\");")
replace("            if(!ok)continue;\n            LOCAL_NOTE(if(repair_priority_broken>=0)", "            if(!ok){nn083_outcome(\"insert_failed\");continue;}\n            LOCAL_NOTE(if(repair_priority_broken>=0)")
replace("            if(same_moves(trial,current)){++duplicates;continue;}", "            if(same_moves(trial,current)){nn083_outcome(\"duplicate\");++duplicates;continue;}")
replace("            if(same_moves(polished,current)){++duplicates;continue;}", "            if(same_moves(polished,current)){nn083_outcome(\"duplicate\");++duplicates;continue;}")
replace("            search_reductions.candidate(polished,current,best.size(),iteration,lns_start,end);", "            search_reductions.candidate(polished,current,best.size(),iteration,lns_start,nn083_clock_active?numeric_limits<double>::infinity():end);")
replace("            if(accept) {\n                LOCAL_NOTE(", "            nn083_outcome(accept?\"accepted\":\"rejected\");\n            if(accept) {\n                LOCAL_NOTE(")
replace("                current=move(polished);rebuild();\n            }\n        }\n    }\n\n};", """                current=move(polished);rebuild();
            }
        }
        }catch(const Nn083WallLimit&){
            if(nn083_clock_active)nn083_finish(current,best,false,"wall_censored");
            throw;
        }catch(const Deadline&){
            if(nn083_clock_active)nn083_finish(current,best,false,"unexpected_deadline");
            throw;
        }catch(...){
            if(nn083_clock_active)nn083_finish(current,best,false,"exception");
            if(nn083_enabled){nn083_failure(nn083_next_phase,-1,-1,false,"parent_exception",1);_exit(97);}
            throw;
        }
        if(nn083_clock_active)nn083_finish(current,best,true,nn083_stop_reason.c_str());
    }

};""")
replace("    for(auto m:best)cout<<board_info.row[m.p]", """    if(nn083_enabled){
        ofstream out(nn083_directory+"/parent.json");
        out<<"{\\\"input_index\\\":"<<nn083_index<<",\\\"snapshots\\\":"<<nn083_next_phase
           <<",\\\"requested_phases\\\":"<<nn083_phases<<",\\\"parent_unchanged\\\":true,\\\"pool_free\\\":"<<state_pool.free_count
           <<",\\\"errors\\\":"<<nn083_errors()<<",\\\"best_T\\\":"<<best.size()<<"}\\n";
        out.close();if(!out)_exit(96);
        if(nn083_errors()!=0||state_pool.free_count!=4){
            nn083_failure(nn083_next_phase,-1,-1,false,"parent_integrity",1);_exit(98);
        }
    }
    for(auto m:best)cout<<board_info.row[m.p]""")
out = ROOT / "adhoc/bin/collect_v083_teacher.cpp"
out.write_text(s)
print(out)
