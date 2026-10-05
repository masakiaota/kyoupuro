#!/usr/bin/env python3
"""Materialize preregistered v402 from frozen v401/v106 components (no execution)."""
from pathlib import Path
import hashlib,json
r=Path(__file__).resolve().parents[2]
a=(r/'src/bin/v401_incremental_cnn.cpp').read_text();b=(r/'src/bin/v106_nn_lns.cpp').read_text()
model=b[b.index('namespace neural_selector {'):b.index('\n#ifdef LOCAL\nstruct NeuralRankStats',b.index('namespace neural_selector {'))]
members=b[b.index('    bool nn106_ready=false;'):b.index('    int nn_choose(const vector<Move>& current,int cooldown)')]
s=a.replace('// v401_incremental_cnn.cpp','// v402_initial_nn_repair.cpp',1).replace('constexpr double LOCAL_TIME_RATIO = 0.80;', 'constexpr double LOCAL_TIME_RATIO = 1.90/1.930;',1)
s=s.replace('namespace neural_rank {',model+'\nnamespace neural_rank {',1)
s=s.replace('class TemporalLNS {','''// The independent identity-vector replay is used before committing any repair.
static bool initial_repair_valid(const vector<Move>& moves) {
    array<vector<int>,max_cells> tower;
    for(int p=0;p<board_info.cell_count;++p)if(board_info.initial[p])tower[p].push_back(int(board_info.initial[p]));
    for(auto m:moves) {
        if(m.p<0||m.p>=board_info.cell_count||m.d>=4||m.k>=tower[m.p].size()||m.l<1||m.l>m.k+1)return false;
        int q=-1;
        for(int step=1;step<=m.l;++step) {
            const int row=board_info.row[m.p]+board_info.di[m.d]*step,col=board_info.col[m.p]+board_info.dj[m.d]*step;
            if(row<0||row>=board_info.N||col<0||col>=board_info.N)return false;
            q=board_info.cell_id[row][col];if(q<0)return false;
        }
        if(tower[q].size()+tower[m.p].size()-m.k>8)return false;
        while(tower[m.p].size()>m.k){tower[q].push_back(tower[m.p].back());tower[m.p].pop_back();}
        for(int p:{int(m.p),q})while(!tower[p].empty()&&tower[p].back()==board_info.nest_code[p])tower[p].pop_back();
    }
    for(const auto& t:tower)if(!t.empty())return false;
    return moves.size()<=100000;
}
#ifdef LOCAL
struct InitialRepairAudit {
    vector<Move> raw,accepted;
    int after=0,ordinary=-1,candidates=0,rank=-1,before_smooth=-1;
    double total_ms=0,selection_ms=0,repair_ms=0,smooth_ms=0,ordinary_ms=0,deadline_overrun_ms=0;
};
static vector<InitialRepairAudit> initial_repair_audit;
#endif

class TemporalLNS {''',1)
s=s.replace('    uint64_t alias_closures=0',members+'''    // A fresh instance is used for each raw seed; these caches never enter LNS.
    void initial_learned_repair(vector<Move>& answer,[[maybe_unused]] int seed_id) {
        initialize_spatial_cuts();rebuild_all(answer);
        const int n=nn_sample(0);
        if(!n){LOCAL_ONLY(trace.count_by("initial_repair_empty",1));return;}
        const uint64_t score_rng=rng.x;
        LOCAL_NOTE(const double selection_start=time_keeper.exact_elapsed_sec();)
        prepare_nn106(answer);time_keeper.check();
        int choice=nn_ranks[0];float highest=-numeric_limits<float>::infinity();
        for(int i=0;i<n;++i) {
            vector<int> ids;
            for(int p:candidates[nn_ranks[i]].ids){
                if(p<0||p>=board_info.cell_count||nn106_index[p]<0)throw logic_error("initial selector ID mismatch");
                ids.push_back(nn106_index[p]);
            }
            const auto value=nn106.score(ids);
            if(!isfinite(value[0])||!isfinite(value[1]))throw logic_error("initial selector nonfinite");
            if(value[0]>highest){highest=value[0];choice=nn_ranks[i];}
        }
        if(rng.x!=score_rng)throw logic_error("initial selector consumed RNG");
        LOCAL_ONLY(trace.count_by("initial_repair_calls",1);trace.count_by("initial_repair_scored",n);
            auto& audit=initial_repair_audit.at(seed_id);audit.candidates=n;audit.rank=choice;
            audit.selection_ms=(time_keeper.exact_elapsed_sec()-selection_start)*1000;);
        time_keeper.check();
        LOCAL_NOTE(const double repair_start=time_keeper.exact_elapsed_sec();
            struct RepairTimer {double& total;double start;~RepairTimer(){total=(time_keeper.exact_elapsed_sec()-start)*1000;}}
                repair_timer{initial_repair_audit.at(seed_id).repair_ms,repair_start};)
        const int tie_mode=rng(12);support_weight=tie_mode==0?0:tie_mode==1?2:tie_mode==2?8:5;
        ride_weight=tie_mode==3?4:1;
        vector<Move> base;State initial;
        if(!extract(answer,candidates[choice].ids,base,initial))return;
        vector<pair<double,int>> order;
        for(int id:candidates[choice].ids) {
            double weight=board_info.floor_dist[id][board_info.nest_pos_by_code[board_info.initial[id]]];
            order.emplace_back(-weight*(0.6+0.8*rng.unit()),id);
        }
        sort(order.begin(),order.end());bool ok=true;
        if(order.size()>=2&&order.size()<=4) {
            vector<Move> rebuilt;ok=insert_two_orders(initial,base,order,int(answer.size())+4,rebuilt);
            if(ok)base=move(rebuilt);
        }else for(auto [unused,id]:order) {
            time_keeper.check();vector<Move> next;
            if(!insert(initial,base,id,int(answer.size())+4-int(base.size()),next)){ok=false;break;}
            base=move(next);initial[id]=board_info.initial[id];
        }
        if(ok) {
            time_keeper.check();
            LOCAL_NOTE(const double smooth_start=time_keeper.exact_elapsed_sec();initial_repair_audit.at(seed_id).before_smooth=base.size();)
            auto polished=smooth_routes(base);time_keeper.check();
            LOCAL_ONLY(initial_repair_audit.at(seed_id).smooth_ms=(time_keeper.exact_elapsed_sec()-smooth_start)*1000;);
            if(!initial_repair_valid(polished))throw logic_error("invalid initial repair");
            time_keeper.check();
            LOCAL_ONLY(trace.count_by("initial_repair_completed",1));
            if(polished.size()<answer.size()) {
                LOCAL_ONLY(trace.count_by("initial_repair_accepted",1);trace.count_by("initial_repair_saved",answer.size()-polished.size());
                    initial_repair_audit.at(seed_id).accepted=polished;);
                answer=move(polished);
            }
        }

    }
    LOCAL_NOTE(int initial_seed_id=-1;)

    uint64_t alias_closures=0''',1)
s=s.replace('            initialize_spatial_cuts();\n            rebuild();initialized=true;', '''            LOCAL_ONLY(if(initial_seed_id>=0){auto& audit=initial_repair_audit.at(initial_seed_id);
                audit.ordinary=current.size();audit.ordinary_ms=(time_keeper.exact_elapsed_sec()-started_at)*1000;});
            initialize_spatial_cuts();
            rebuild();initialized=true;''',1)
s=s.replace('        searches.back()->random_state=rng.next();','        searches.back()->random_state=rng.next();\n        LOCAL_ONLY(searches.back()->initial_seed_id=id);',1)
anchor='int main() {'
phase='''// Preserve the input-seeded main RNG; preprocessing gets a fixed private stream.
static void initial_learned_repairs(InitialSolutions& pool,vector<Move>& best) {
    const double begin=time_keeper.exact_elapsed_sec();
    const double deadline=min(begin+0.12*PROGRAM_TIME_LIMIT_SEC,0.40*PROGRAM_TIME_LIMIT_SEC);
    const uint64_t saved_rng=rng.x;const double saved_limit=time_keeper.time_limit_sec;
    struct Restore {uint64_t random;double limit;~Restore(){rng.x=random;time_keeper.time_limit_sec=limit;}} restore{saved_rng,saved_limit};
    LOCAL_ONLY(initial_repair_audit.resize(pool.entries.size());trace.count_by("initial_repair_raw_seeds",pool.entries.size()););
    for(size_t i=0;i<pool.entries.size();++i) {
        auto& seed=pool.entries[i];const double start=time_keeper.exact_elapsed_sec();
        LOCAL_ONLY(initial_repair_audit[i].raw=seed.moves;);
        time_keeper.time_limit_sec=min(deadline,start+0.04*PROGRAM_TIME_LIMIT_SEC);
        rng.x=saved_rng^0x4021069e3779b9ULL^(uint64_t(i+1)*0x9e3779b97f4a7c15ULL);if(!rng.x)rng.x=1;
        try {time_keeper.check();TemporalLNS repair;repair.initial_learned_repair(seed.moves,int(i));}
        catch(const Deadline&){LOCAL_ONLY(trace.count_by("initial_repair_deadlines",1));}
        if(seed.moves.size()<best.size())best=seed.moves;
        LOCAL_ONLY(auto& audit=initial_repair_audit[i];audit.after=seed.moves.size();
            audit.total_ms=(time_keeper.exact_elapsed_sec()-start)*1000;
            audit.deadline_overrun_ms=max(0.0,time_keeper.exact_elapsed_sec()-time_keeper.time_limit_sec)*1000;);
    }
    // Keep original raw-seed order and race policy; only the seed moves change.
    LOCAL_ONLY(trace.add_time_ms("initial_repair",(time_keeper.exact_elapsed_sec()-begin)*1000));
}
#ifdef LOCAL
static void print_initial_repair_audit() {
    for(size_t i=0;i<initial_repair_audit.size();++i) {
        const auto& a=initial_repair_audit[i];const string prefix="initial_repair_seed_"+to_string(i);
        trace.count_by(prefix+"_raw",a.raw.size());trace.count_by(prefix+"_after",a.after);
        trace.count_by(prefix+"_before_smooth",a.before_smooth);trace.count_by(prefix+"_ordinary",a.ordinary);trace.count_by(prefix+"_scored",a.candidates);trace.count_by(prefix+"_rank",a.rank);
        trace.add_time_ms(prefix+"_total",a.total_ms);trace.add_time_ms(prefix+"_selection",a.selection_ms);
        trace.add_time_ms(prefix+"_smooth",a.smooth_ms);trace.add_time_ms(prefix+"_repair",a.repair_ms);trace.add_time_ms(prefix+"_ordinary",a.ordinary_ms);
        trace.add_time_ms(prefix+"_deadline_overrun",a.deadline_overrun_ms);
        for(auto label:{string("raw"),string("accepted")}) {
            const auto& moves=label=="raw"?a.raw:a.accepted;
            if(moves.empty())continue;
            cerr<<"[initial_repair."<<label<<"] "<<i<<' '<<moves.size()<<'\\n';
            for(auto m:moves)cerr<<board_info.row[m.p]<<' '<<board_info.col[m.p]<<' '<<int(m.k)<<' '<<board_info.dirs[m.d]<<' '<<int(m.l)<<'\\n';
            cerr<<"[initial_repair.end]\\n";
        }
    }
}
#endif

'''
s=s.replace(anchor,phase+anchor,1)
s=s.replace('    neural_extra_starts(input,initial_solutions,best);','    neural_extra_starts(input,initial_solutions,best);\n    initial_learned_repairs(initial_solutions,best);',1)
s=s.replace('    cerr<<"v311_nn_first_ops="','    LOCAL_ONLY(print_initial_repair_audit());\n    cerr<<"v311_nn_first_ops="',1)
(r/'src/bin/v402_initial_nn_repair.cpp').write_text(s)
out=r/'results/analysis/v402';out.mkdir(parents=True,exist_ok=True)
(out/'component_hashes.json').write_text(json.dumps({k:hashlib.sha256(v.encode()).hexdigest() for k,v in [('v401',a),('v106',b),('v402',s),('selector_namespace',model),('selector_members',members)]},indent=2))
