#!/usr/bin/env python3
"""Generate old/transplant numerical probes; never execute the full solver."""
from pathlib import Path
r=Path(__file__).resolve().parents[2]
body=r'''
struct NeuralRankProbe {
 static void run(const char* path) {
    board_info.read();state_pool.prepare(board_info.cell_count);
    time_keeper.start_=chrono::steady_clock::now();time_keeper.time_limit_sec=1e9;rng.x=106402;
    vector<Move> moves;ifstream f(path);int row,col,k,l;char d;
    while(f>>row>>col>>k>>d>>l) {
        int direction=0;while(direction<4&&board_info.dirs[direction]!=d)++direction;
        moves.push_back({int16_t(board_info.cell_id[row][col]),uint8_t(k),uint8_t(direction),uint8_t(l)});
    }
    {
        TemporalLNS lns;lns.initialize_spatial_cuts();lns.rebuild_all(moves);
        const uint64_t saved=rng.x;
        const int n=lns.nn_sample(0);lns.prepare_nn106(moves);
        int chosen=-1;float highest=-numeric_limits<float>::infinity();
        for(int i=0;i<n;++i) {
            const int rank=lns.nn_ranks[i];vector<int> ids;
            for(int p:lns.candidates[rank].ids){if(lns.nn106_index[p]<0)throw logic_error("ID map");ids.push_back(lns.nn106_index[p]);}
            auto score=lns.nn106.score(ids);
            cout<<rank<<' '<<hexfloat<<score[0]<<' '<<score[1];for(int id:ids)cout<<' '<<id;cout<<'\n';
            if(score[0]>highest){highest=score[0];chosen=rank;}
        }
        cout<<"chosen "<<chosen<<" RNG "<<(rng.x==saved)<<'\n';
        if(rng.x!=saved)throw logic_error("RNG modified");
    }
    if(state_pool.free_count!=4)throw logic_error("pool leak");
 }
};
int main(int argc,char**argv){if(argc!=2)return 2;NeuralRankProbe::run(argv[1]);}
'''
for version,name in [('106','v106_nn_lns'),('402','v402_initial_nn_repair')]:
 p=r/f'adhoc/bin/v402_selector_reference_{version}.cpp'
 p.write_text(f'// {p.name}\n#define AHC072_NN_PROBE\n#define main unused_solver_main\n#include "../../src/bin/{name}.cpp"\n#undef main\n'+body)
