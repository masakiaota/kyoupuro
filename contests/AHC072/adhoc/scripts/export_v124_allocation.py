#!/usr/bin/env python3
"""固定モデルをv113の計画配分へ組み込む。"""
import json
from build_v124_allocation import ROOT, RUN, BASE, BASE_SHA, grow_tracking
from check_v113_integrated import block
from v089_data import sha

def build_candidate(model_path,name):
    assert sha(BASE)==BASE_SHA
    parameters=json.loads(model_path.read_text())['parameters']
    def literal(x):
        if isinstance(x,list):return '{'+','.join(literal(v) for v in x)+'}'
        return float(x).hex()+'f'
    model='''namespace allocation_policy {
static constexpr float weights[18][16]='''+literal([list(v) for v in zip(*parameters['0.weight'])])+''';
static constexpr float bias[16]='''+literal(parameters['0.bias'])+''';
static constexpr float output[16]='''+literal(parameters['2.weight'][0])+''';
static constexpr float intercept='''+literal(parameters['2.bias'][0])+''';
float predict(const array<float,18>& f) {
    array<float,16> h;copy(bias,bias+16,h.begin());
    for(int i=0;i<18;++i)for(int j=0;j<16;++j)h[j]+=weights[i][j]*f[i];
    float result=intercept;for(int j=0;j<16;++j)result+=output[j]*max(0.f,h[j]);return result;
}
}
'''
    text=BASE.read_text();grow=grow_tracking(text)
    rank='''    auto allocation_rank=[&](vector<int>& ids) {
        const double now=time_keeper.exact_elapsed_sec();
        int shortest=INT_MAX;for(const auto& entry:pool.entries)shortest=min(shortest,int(entry.moves.size()));
        array<double,5> values{};
        for(int id:ids){const int current=int(pool.entries[id].moves.size());
            const auto f=allocation_policy::features(allocation_tracks[id],current,shortest,now,start,end);
            const double gain=clamp(20.0*allocation_policy::predict(f),0.0,double(current));
            if(!isfinite(gain))throw logic_error("invalid allocation prediction");
            values[id]=current-gain;
        }
        sort(ids.begin(),ids.end(),[&](int a,int b){
            if(values[a]!=values[b])return values[a]<values[b];
            return pool.entries[a].moves.size()!=pool.entries[b].moves.size()?pool.entries[a].moves.size()<pool.entries[b].moves.size():a<b;
        });
        LOCAL_ONLY(trace.count_by("allocation_rank_calls",1);
                   trace.count_by("allocation_selected_longer",int(pool.entries[ids[0]].moves.size())>shortest);
                   trace.add_time_ms("allocation_prediction",(time_keeper.exact_elapsed_sec()-now)*1000));
    };
'''
    grow=grow.replace('    try {\n        // 最初の2段階',rank+'    try {\n        // 最初の2段階',1)
    old='''            sort(active.begin(),active.end(),[&](int a,int b) {
                const size_t left=pool.entries[a].moves.size(),right=pool.entries[b].moves.size();
                return left!=right?left<right:a<b;
            });'''
    assert grow.count(old)==1;grow=grow.replace(old,'            allocation_rank(active);',1)
    rerank=block(grow,'auto rerank=[&]')
    grow=grow.replace(rerank,'auto rerank=[&] { allocation_rank(active); }',1)
    old='''const bool challenge=ReactiveRaceStats::challenge(step,
                int(pool.entries[active[0]].moves.size()),int(pool.entries[active[1]].moves.size()));'''
    assert grow.count(old)==1
    grow=grow.replace(old,'const bool challenge=pool.entries[active[0]].moves.size()>pool.entries[active[1]].moves.size();',1)
    grow=grow.replace('            reactive_race_stats.blocked_challenges+=step%4==3&&!challenge;\n','',1)
    grow=grow.replace('const int id=active[challenge?1:0];','const int id=active[0];',1)
    helper=(ROOT/'adhoc/scripts/v124_features.cpp.txt').read_text()+'\n'+model
    text=text.replace(block(text,'void grow_initial_solutions('),helper+'\n'+grow,1)
    predicate=block(text,'static bool challenge(');text=text.replace(predicate,'',1)
    target=ROOT/'adhoc/bin'/(name+'.cpp');target.write_text('// '+target.name+'\n'+text.split('\n',1)[1])
    return target

