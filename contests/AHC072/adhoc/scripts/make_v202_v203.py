#!/usr/bin/env python3
"""両親へ同じ初期解選抜を適用する。solverは実行しない。"""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[2]
PAIRS = (("v076_relative_tuned", "v202_initial_race"),
         ("v201_floor_nn_selector", "v203_floor_nn_initial_race"))
CHANGES = []

STATS = """attempts insertions accepted improvements shortcuts block_attempts block_accepted
block_improvements uphill restarts flexible_attempts flexible_accepted flexible_improvements
flexible_saved reorder_attempts reorder_improvements reorder_saved duplicates paired_attempts
paired_accepted paired_improvements paired_saved backward_attempts backward_improvements
backward_saved paired_seconds block_seconds build_seconds""".split()

RACE = r'''
// 構築直後には同手数でも、その後の短縮余地が異なり得るため別の操作列を残す。
struct InitialSolutions {
    struct Entry { vector<Move> moves; int origin; };
    vector<Entry> entries;
    void offer(const vector<Move>& moves,int origin) {
        LOCAL_ONLY(trace.count_by("race_offered",1));
        for(const auto& entry:entries) {
            if(entry.moves.size()!=moves.size())continue;
            bool same=true;
            for(size_t i=0;i<moves.size();i++) {
                const auto a=entry.moves[i],b=moves[i];
                if(a.p!=b.p||a.k!=b.k||a.d!=b.d||a.l!=b.l){same=false;break;}
            }
            if(same){LOCAL_ONLY(trace.count_by("race_duplicates",1));return;}
        }
        const auto at=upper_bound(entries.begin(),entries.end(),moves.size(),
            [](size_t size,const Entry& entry){return size<entry.moves.size();});
        if(at==entries.end()&&entries.size()==4)return;
        entries.insert(at,Entry{moves,origin});
        if(entries.size()>4)entries.pop_back();
        LOCAL_ONLY(trace.count_by("race_retained",1));
    }
    size_t construction_cap() const {
        // 最長候補と同じ手数まで完成させ、別の初期解を選抜へ渡せるようにする。
        return entries.back().moves.size()+1;
    }
};

void grow_initial_solutions(vector<Move>& best,InitialSolutions& pool,TemporalLNS& total) {
    const double start=time_keeper.exact_elapsed_sec(),end=PROGRAM_TIME_LIMIT_SEC;
    const double budget=max(0.0,end-start);
    const int count=int(pool.entries.size());
    if(count==0)throw logic_error("empty initial solution pool");
    vector<unique_ptr<TemporalLNS>> searches;
    vector<int> active(count);iota(active.begin(),active.end(),0);
    for(int id=0;id<count;id++) {
        searches.push_back(make_unique<TemporalLNS>());
        searches.back()->random_state=rng.next();
        LOCAL_ONLY(
            trace.count_by("race_seed_"+to_string(id)+"_initial_ops",pool.entries[id].moves.size());
            trace.count_by("race_seed_"+to_string(id)+"_origin",pool.entries[id].origin);
        );
    }
    total.started_at=start;
    LOCAL_ONLY(trace.count_by("race_seed_count",count));
    auto grow=[&](int id,double until,[[maybe_unused]] int round) {
        LOCAL_NOTE(const double began=time_keeper.exact_elapsed_sec();const int previous_attempts=searches[id]->attempts;)
        auto record=[&]() {
            LOCAL_ONLY(
                const string prefix="race_round_"+to_string(round);
                trace.count_by(prefix+"_grown",1);
                trace.count_by(prefix+"_seed_"+to_string(id)+"_ops",pool.entries[id].moves.size());
                trace.count_by(prefix+"_seed_"+to_string(id)+"_attempts",searches[id]->attempts-previous_attempts);
                trace.add_time_ms(prefix,(time_keeper.exact_elapsed_sec()-began)*1000.0);
            );
        };
        try { searches[id]->optimize(pool.entries[id].moves,until,start,end); }
        catch(...) {
            if(pool.entries[id].moves.size()<best.size())best=pool.entries[id].moves;
            record();
            throw;
        }
        if(pool.entries[id].moves.size()<best.size())best=pool.entries[id].moves;
        record();
    };
    auto summarize=[&]() {
        for(int id=0;id<count;id++) {
            total.accumulate(*searches[id]);
            LOCAL_ONLY(trace.count_by("race_seed_"+to_string(id)+"_final_ops",pool.entries[id].moves.size()));
        }
    };
    try {
        // 最初の2段階に各15%を使い、最短候補の継続探索へ70%を残す。
        for(int round=0;round<2&&active.size()>1;round++) {
            const double round_end=start+budget*(0.15*(round+1));
            for(size_t pos=0;pos<active.size();pos++) {
                const double now=time_keeper.exact_elapsed_sec();
                const double until=now+max(0.0,round_end-now)/double(active.size()-pos);
                grow(active[pos],until,round);
            }
            sort(active.begin(),active.end(),[&](int a,int b) {
                const size_t left=pool.entries[a].moves.size(),right=pool.entries[b].moves.size();
                return left!=right?left<right:a<b;
            });
            active.resize(round==0?(active.size()+1)/2:1);
            LOCAL_ONLY(trace.count_by("race_round_"+to_string(round)+"_survivors",active.size()));
        }
        const int winner=active.front();
        LOCAL_ONLY(
            trace.count_by("race_winner_initial_rank",winner);
            trace.count_by("race_winner_changed",winner!=0);
            trace.count_by("race_selection_saved",int(pool.entries[0].moves.size())-int(pool.entries[winner].moves.size()));
        );
        grow(winner,end,2);
        // 終了後の処理にも、選ばれた探索の乱数列を引き継ぐ。
        rng.x=searches[winner]->random_state;
    } catch(...) {summarize();throw;}
    summarize();
}

'''


def replace(source, old, new):
    assert source.count(old) == 1, (old[:90], source.count(old))
    CHANGES.append({"old": old, "new": new, "offset": source.index(old)})
    return source.replace(old, new)


def make(parent, child):
    CHANGES.clear()
    source = (ROOT / f"src/bin/{parent}.cpp").read_text()
    source = replace(source, f"// {parent}.cpp", f"// {child}.cpp")
    source = replace(source, "    double lns_start=0;", """    double lns_start=0;
    vector<Move> current;
    int stagnant=0;
    bool initialized=false;
    double active_seconds=0;""")
    source = replace(source, "    void optimize(vector<Move>& best) {", """    uint64_t random_state=1;
    void accumulate(const TemporalLNS& other) {
""" + "".join(f"        {name}+=other.{name};\n" for name in STATS) + """    }
    void optimize(vector<Move>& best,double slice_end,double search_start,double search_end) {
        const double slice_begin=time_keeper.exact_elapsed_sec();
        // 中断期間を候補自身の費用へ含めず、乱数も候補ごとに続きから使う。
        struct SliceScope {
            double& seconds;uint64_t& state;uint64_t previous;double begin;
            ~SliceScope(){seconds+=time_keeper.exact_elapsed_sec()-begin;state=rng.x;rng.x=previous;}
        } scope{active_seconds,random_state,rng.x,slice_begin};
        rng.x=random_state;
        lns_start=search_start;
        auto rebuild=[&](){double t=time_keeper.exact_elapsed_sec();make_candidates(current);build_seconds+=time_keeper.exact_elapsed_sec()-t;};
        LOCAL_ONLY(trace.count_by("race_resumes",initialized));""")
    if parent.startswith("v201"):
        source = replace(source, '        LOCAL_NOTE(trace.count_by("selector_nn_enabled",use_nn); trace.count_by("selector_M",board_info.M);)\n', "")
    source = replace(source, "        started_at=time_keeper.exact_elapsed_sec();\n        auto current=smooth_routes(best);", "        if(!initialized) {\n        started_at=time_keeper.exact_elapsed_sec();\n        current=smooth_routes(best);")
    source = replace(source, "search_reductions.cheap(current,PROGRAM_TIME_LIMIT_SEC);", "search_reductions.cheap(current,min(slice_end,search_end));")
    source = replace(source, "search_reductions.cheap(polished,PROGRAM_TIME_LIMIT_SEC);", "search_reductions.cheap(polished,min(slice_end,search_end));")
    source = replace(source, """        initialize_spatial_cuts();
        auto rebuild=[&](){double t=time_keeper.exact_elapsed_sec();make_candidates(current);build_seconds+=time_keeper.exact_elapsed_sec()-t;};
        rebuild();lns_start=time_keeper.exact_elapsed_sec();int stagnant=0;
        const double end=LOCAL_SECONDS(1.900);
        double temperature=lns_start_temperature;
        while(time_keeper.exact_elapsed_sec()<end) {""", """        initialize_spatial_cuts();
        rebuild();initialized=true;
        }
        const double end=search_end;
        // 切り替え時の再加熱を避け、全候補を同じ全体進捗で冷却する。
        const double initial_progress=clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(PROGRAM_TIME_LIMIT_SEC*(0.02/1.90),end-lns_start),0.0,1.0);
        double temperature=lns_start_temperature*pow(lns_end_temperature/lns_start_temperature,initial_progress);
        while(time_keeper.exact_elapsed_sec()<min(slice_end,end)) {""")
    source = replace(source, "paired_seconds<0.11*max(LOCAL_SECONDS(0.03),time_keeper.exact_elapsed_sec()-lns_start)", "paired_seconds<0.11*max(PROGRAM_TIME_LIMIT_SEC*(0.03/1.90),active_seconds+time_keeper.exact_elapsed_sec()-slice_begin)")
    source = replace(source, "block_seconds<0.22*max(LOCAL_SECONDS(0.03),time_keeper.exact_elapsed_sec()-lns_start)", "block_seconds<0.22*max(PROGRAM_TIME_LIMIT_SEC*(0.03/1.90),active_seconds+time_keeper.exact_elapsed_sec()-slice_begin)")
    source = replace(source, "int main() {", RACE + "int main() {")
    source = replace(source, "    vector<Move> best;size_t baseline=0;", "    vector<Move> best;size_t baseline=0;\n    InitialSolutions initial_solutions;")
    source = replace(source, "            ConstructionState w=tree_baseline(v);", "            ConstructionState w=tree_baseline(v);\n            initial_solutions.offer(w.moves,-1-v);")
    source = replace(source, "    EmptyDP potential;", "    initial_solutions.offer(best,-4);\n    EmptyDP potential;")
    source = replace(source, "            Constructor solver(potential,attempts++,end,best.size());", "            Constructor solver(potential,attempts++,end,initial_solutions.construction_cap());")
    source = replace(source, "            if(!result.E){completed++;if(result.moves.size()<best.size())best=move(result.moves);}", "            if(!result.E){completed++;initial_solutions.offer(result.moves,attempts-1);if(result.moves.size()<best.size())best=move(result.moves);}")
    source = replace(source, "    try {lns.optimize(best);}", "    try {grow_initial_solutions(best,initial_solutions,lns);}")
    if parent.startswith("v201"):
        source = replace(source, "    TemporalLNS lns;", "    LOCAL_ONLY(trace.count_by(\"selector_nn_enabled\",nn_selector_choice(nn_selector_features()));trace.count_by(\"selector_M\",board_info.M));\n    TemporalLNS lns;")
    first = source.index("        if(!initialized) {\n") + len("        if(!initialized) {\n")
    last = source.index("        rebuild();initialized=true;\n", first) + len("        rebuild();initialized=true;\n")
    block = source[first:last]
    source = replace(source, block, "".join("    " + line for line in block.splitlines(True)))
    # 変更を逆順に戻せば、元の位置で親ソースを復元できる。
    audit = ROOT / "adhoc/v202_v203_audit"
    audit.mkdir(parents=True, exist_ok=True)
    (audit / f"{child}_changes.json").write_text(json.dumps(CHANGES, ensure_ascii=False, indent=2) + "\n")
    (ROOT / f"src/bin/{child}.cpp").write_text(source)


if __name__ == "__main__":
    for parent, child in PAIRS:
        assert not (ROOT / f"src/bin/{child}.cpp").exists(), child
        make(parent, child)
        print(f"{parent} -> {child}")
