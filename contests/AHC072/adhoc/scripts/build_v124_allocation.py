#!/usr/bin/env python3
"""v113の計画配分だけを観測・置換する。LNS本体と初期NNは保持する。"""
import json
from pathlib import Path
from check_v113_integrated import block
from v089_data import ROOT, save, sha, now

RUN=ROOT/'results/nn_rank/v124/20261004_plan_allocation_studio'
BASE=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
BASE_SHA='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'


def grow_tracking(text):
    grow=block(text,'void grow_initial_solutions(')
    grow=grow.replace('    auto grow=[&]', '''    vector<allocation_policy::Track> allocation_tracks(count);
    for(int id=0;id<count;++id){allocation_tracks[id].raw=int(pool.entries[id].moves.size());allocation_tracks[id].last_end=start;}
    auto grow=[&]''',1)
    grow=grow.replace('        auto record=[&]() {','''        const double allocation_begin=time_keeper.exact_elapsed_sec();
        const int allocation_before=int(pool.entries[id].moves.size());
        auto record=[&]() {
            auto& track=allocation_tracks[id];const double finished=time_keeper.exact_elapsed_sec();
            track.last_gain=allocation_before-int(pool.entries[id].moves.size());
            track.last_duration=finished-allocation_begin;track.used+=track.last_duration;track.last_end=finished;
            track.attempts=searches[id]->attempts;track.improvements=searches[id]->improvements;''',1)
    return grow


def build_collector():
    assert sha(BASE)==BASE_SHA
    text=BASE.read_text();grow=grow_tracking(text)
    grow=grow.replace('searches.back()->random_state=rng.next();','searches.back()->random_state=rng.next()^v124_mix(v124_repeat);',1)
    grow=grow.replace('            sort(active.begin(),active.end(),[&](int a,int b) {',
                      '            v124_collect(round,pool,searches,allocation_tracks,start,end);\n            sort(active.begin(),active.end(),[&](int a,int b) {',1)
    grow=grow.replace('int last_leader=active.front(),step=0;','int last_leader=active.front(),step=0;bool observed_middle=false;',1)
    grow=grow.replace('grow(id,min(late_end,time_keeper.exact_elapsed_sec()+quantum),3+step);++step;',
                      '''grow(id,min(late_end,time_keeper.exact_elapsed_sec()+quantum),3+step);++step;
            if(!observed_middle&&time_keeper.exact_elapsed_sec()>=start+budget*.55){observed_middle=true;v124_collect(2,pool,searches,allocation_tracks,start,end);}''',1)
    grow=grow.replace('        rerank();const int winner=active.front();','        v124_collect(3,pool,searches,allocation_tracks,start,end);\n        rerank();const int winner=active.front();',1)
    helper=(ROOT/'adhoc/scripts/v124_features.cpp.txt').read_text()+'\n'+(ROOT/'adhoc/scripts/v124_collect.cpp.txt').read_text()
    text=text.replace(block(text,'void grow_initial_solutions('),helper+'\n'+grow,1)
    text=text.replace('int main() {','int v124_parent_main() {',1)
    main=block(text,'int v124_parent_main() {')
    text=text.replace(main,main[:-1]+'    return 0;\n}',1)
    text+="""
int main(int argc,char** argv) {
    if(argc!=3)return 2;
    v124_repeat=stoull(argv[1]);v124_labels.open(argv[2]);if(!v124_labels)return 3;
    const int code=v124_parent_main();v124_labels.close();return code;
}
"""
    target=ROOT/'adhoc/bin/collect_v124_allocation.cpp'
    target.write_text('// '+target.name+'\n#include <unistd.h>\n#include <sys/wait.h>\n'+text.split('\n',1)[1])
    return target


if __name__=='__main__':print(build_collector())
