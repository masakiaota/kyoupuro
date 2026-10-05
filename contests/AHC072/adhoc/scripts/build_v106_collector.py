#!/usr/bin/env python3
"""v105の探索経路を保ったまま、教師採取だけの観測点を追加する。"""
from pathlib import Path
from v089_data import ROOT, sha


def build():
    parent=ROOT/'src/bin/v105_nn_lns.cpp'
    assert sha(parent)=='1f55b1534b9e912a2cc8db3386cd6c1fe9515e417d0462a9300562b75a51ae58'
    original=parent.read_text()
    source=original.replace('// v105_nn_lns.cpp','// v106_probe_base.cpp',1)
    declaration='''struct V106Snapshot {
    vector<Move> moves,best;int iteration,stagnant;double progress;
};
static vector<V106Snapshot> v106_snapshots;

'''
    source=source.replace('class TemporalLNS {',declaration+'class TemporalLNS {',1)
    anchor='            time_keeper.check();++iteration;++stagnant;'
    assert source.count(anchor)==1
    source=source.replace(anchor,'''            // 観測用の複製だけを行い、探索の乱数と操作列を変えない。
            if(iteration+1==128||iteration+1==512||iteration+1==2048) {
                const double p=clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(LOCAL_SECONDS(0.02),end-lns_start),0.0,1.0);
                v106_snapshots.push_back({current,best,iteration+1,stagnant,p});
            }
'''+anchor,1)
    (ROOT/'adhoc/bin/v106_probe_base.cpp').write_text(source)
    old=(ROOT/'adhoc/bin/collect_v087_rewards.cpp').read_text()
    common=old[old.index('    struct Trial'):old.index('    static void run(')]
    nn_prefix=original[original.index('int main() {')+len('int main() {'):original.index('    const size_t baseline=best.size();')]
    begin=nn_prefix.index('    if(result.E!=0){')
    nn_prefix=nn_prefix[:begin]+'    if(result.E!=0)throw logic_error("NN did not complete training input");\n'
    body=r'''
    static void print_plan(const vector<Move>& plan) {
        cout<<'[';int n=0;for(auto m:plan){if(n++)cout<<',';cout<<'['<<m.p<<','<<int(m.k)<<','<<int(m.d)<<','<<int(m.l)<<']';}cout<<']';
    }
    static void run(uint64_t case_id,bool check) {
NN_PREFIX
        verify(best);v106_snapshots.push_back({best,best,0,0,0.});
        InitialSolutions initial_solutions;initial_solutions.offer(best,-105);
        time_keeper.time_limit_sec=LOCAL_SECONDS(1.930);
        TemporalLNS search;
        try {grow_initial_solutions(best,initial_solutions,search);}
        catch(const Deadline&) {} // 教師採取の主探索枠で到達した観測点を使う。
        time_keeper.time_limit_sec=1e9;
        vector<int> ids;array<int,max_cells> index;index.fill(-1);
        for(int p=0;p<board_info.cell_count;p++)if(board_info.initial[p]){index[p]=int(ids.size());ids.push_back(p);}
        if(int(ids.size())!=board_info.M)throw logic_error("individual indexing failed");
        int row=0;
        for(const auto& snapshot:v106_snapshots) {
            const auto& current=snapshot.moves;verify(current);
            const uint64_t seed=mix(106001ULL^(case_id<<32)^uint64_t(row));rng.x=seed;
            TemporalLNS lns;lns.initialize_spatial_cuts();lns.make_candidates(current);
            const int sampled=lns.nn_sample(0);
            if(sampled<1||sampled>32)throw logic_error("invalid candidate count");
            vector<Candidate> choices;map<vector<int>,int> seen;
            int chosen=0;float selected_value=numeric_limits<float>::infinity();
            const bool use_nn=nn_selector_choice(nn_selector_features());
            for(int k=0;k<sampled;k++) {
                auto& removal=lns.candidates[lns.nn_ranks[k]];
                lns.refresh_regular_score(removal);
                auto selected=removal.ids;sort(selected.begin(),selected.end());
                if(selected.empty()||selected.size()>12||adjacent_find(selected.begin(),selected.end())!=selected.end())throw logic_error("invalid selected set");
                for(int id:selected)if(id<0||id>=board_info.cell_count||index[id]<0)throw logic_error("unknown individual");
                auto [it,fresh]=seen.emplace(selected,int(choices.size()));
                if(fresh){Candidate c;c.ids=selected;c.sources=1;choices.push_back(move(c));}
                if(use_nn) {
                    const float value=neural_rank::predict(lns.nn_features(removal,current,snapshot.best,snapshot.stagnant,snapshot.progress));
                    if(value<selected_value){selected_value=value;chosen=it->second;}
                }
            }
            choices[chosen].sources|=4|8;
            for(auto& candidate:choices)for(int r=0;r<8;r++)
                candidate.trials[r]=attempt(lns,current,candidate.ids,mix(seed^uint64_t(r+1)*0xd1b54a32d192ed03ULL));
            if(check&&row==0) {
                const auto repeated=attempt(lns,current,choices[0].ids,mix(seed^0xd1b54a32d192ed03ULL));
                const auto& first=choices[0].trials[0];
                if(repeated.T!=first.T||repeated.hash!=first.hash||!lns.same_moves(repeated.plan,first.plan))throw logic_error("repair replay changed");
            }
            cout<<"{\"row\":"<<row<<",\"iteration\":"<<snapshot.iteration<<",\"progress\":"<<snapshot.progress
                <<",\"current_T\":"<<current.size()<<",\"seed\":"<<seed<<",\"check_passed\":"<<(check&&row==0?"true":"null")
                <<",\"baseline_index\":"<<chosen<<",\"baseline_nn\":"<<(use_nn?"true":"false")<<",\"current\":";
            print_plan(current);cout<<",\"candidates\":[";
            int written=0;
            for(const auto& c:choices) {
                if(written++)cout<<',';cout<<"{\"ids\":[";
                for(int i=0;i<int(c.ids.size());i++){if(i)cout<<',';cout<<index[c.ids[i]];}
                cout<<"],\"sources\":"<<c.sources<<",\"outcome_T\":[";
                for(int r=0;r<8;r++){if(r)cout<<',';cout<<c.trials[r].T;}
                cout<<"],\"hashes\":[";for(int r=0;r<8;r++){if(r)cout<<',';cout<<c.trials[r].hash;}
                cout<<"],\"milliseconds\":[";for(int r=0;r<8;r++){if(r)cout<<',';cout<<c.trials[r].ms;}cout<<"]}";
            }
            cout<<"],\"audit_plan\":";bool saved=false;
            if(row==0)for(const auto& c:choices)for(const auto& t:c.trials)if(!saved&&t.T>0){print_plan(t.plan);saved=true;}
            if(!saved)cout<<"null";cout<<"}\n"<<flush;++row;
        }
        if(state_pool.free_count!=4)throw logic_error("collector state leak");
    }
};
int main(int argc,char** argv) {
    if(argc!=3)return 2;
    NeuralRankProbe::run(stoull(argv[1]),string(argv[2])=="check");
}
'''.replace('NN_PREFIX',nn_prefix)
    header='// collect_v106_rewards.cpp\n#define AHC072_NN_PROBE\n#define main unused_v106_main\n#include "v106_probe_base.cpp"\n#undef main\n\nstruct NeuralRankProbe {\n'
    (ROOT/'adhoc/bin/collect_v106_rewards.cpp').write_text(header+common+body)


if __name__=='__main__':build()
