#!/usr/bin/env python3
"""固定した短縮量モデルをv105の通常候補選択へ組み込む。"""
import argparse
import json
from pathlib import Path
from v089_data import ROOT, sha


def declaration(name,values):
    shape='';part=values
    while isinstance(part,list):shape+=f'[{len(part)}]';part=part[0]
    def literal(x):
        if isinstance(x,list):return '{'+','.join(map(literal,x))+'}'
        value=format(float(x),'.9g')
        if '.' not in value and 'e' not in value:value+='.'
        return value+'f'
    return f'constexpr float {name}{shape}={literal(values)};\n'


def build(model_path,output):
    model=json.loads(model_path.read_text());weights=declaration('mean',model['mean'])+declaration('scale',model['scale'])
    for name,value in model['parameters'].items():weights+=declaration(name.replace('.','_'),value)
    original=(ROOT/'src/bin/v105_nn_lns.cpp').read_text()
    assert sha(ROOT/'src/bin/v105_nn_lns.cpp')=='1f55b1534b9e912a2cc8db3386cd6c1fe9515e417d0462a9300562b75a51ae58'
    support=(ROOT/'adhoc/scripts/v086_proposal.cpp.txt').read_text()
    context=support[:support.index('        // 選択済み集合に依存しない')]
    context=context.replace('namespace neural_proposal','namespace neural_selector').replace('// V086_WEIGHTS',weights)
    context=context.replace('h,choose_base','h').replace('h.resize(M);choose_base.resize(M);','h.resize(M);')
    context+='''    }
    array<float,2> score(const vector<int>& selected) const {
        const int M=int(h.size()),n=int(selected.size());
        if(n<1||n>12)throw logic_error("invalid selector set");
        array<float,102> x{};copy(global.begin(),global.end(),x.begin());
        for(int k=64;k<96;k++)x[k]=-numeric_limits<float>::infinity();
        for(int i:selected) {
            if(i<0||i>=M)throw logic_error("selector individual out of range");
            for(int k=0;k<32;k++){x[32+k]+=h[i][k]/n;x[64+k]=max(x[64+k],h[i][k]);}
            for(int r=0;r<5;r++)for(int j:selected)x[96+r]+=edges[(r*M+i)*M+j]/n;
        }
        x[101]=n/12.0f;array<float,2> result{head_2_bias[0],head_2_bias[1]};
        for(int o=0;o<64;o++) {
            float value=head_0_bias[o];
            for(int k=0;k<102;k++)value+=head_0_weight[o][k]*x[k];
            value=max(0.0f,value);
            for(int k=0;k<2;k++)result[k]+=head_2_weight[k][o]*value;
        }
        if(!isfinite(result[0])||!isfinite(result[1]))throw logic_error("nonfinite selector prediction");
        return result;
    }
};
}
'''
    members=support.split('// V086_MEMBERS\n')[1]
    members=members[:members.index('    Removal nn086_propose')]
    members=members.replace('    int nn086_regular=0;\n','').replace('neural_proposal::Context','neural_selector::Context')
    feature=(ROOT/'adhoc/scripts/v084_set_support.cpp.txt').read_text()
    feature=feature[feature.index('    void prepare_nn084_items'):feature.index('    pair<array<float,8>')]
    feature='    bool nn086_items_ready=false;\n    array<array<double,20>,max_cells> nn086_items{};\n'+feature.replace('nn084','nn086')
    members=members.replace('    // V086_ITEM_FEATURES',feature).replace('nn086','nn106')
    members=members.replace('        prepare_nn106_items(current);','        ensure_history(current);\n        prepare_nn106_items(current);',1)
    choose='''    int nn_choose(const vector<Move>& current,int cooldown) {
        LOCAL_NOTE(const auto started=chrono::steady_clock::now();)
        const int n=nn_sample(cooldown);
        if(!n)throw logic_error("no eligible selector candidates");
        prepare_nn106(current);
        LOCAL_NOTE(const auto encoded=chrono::steady_clock::now();)
        int chosen=nn_ranks[0];float highest=-numeric_limits<float>::infinity();
        for(int i=0;i<n;i++) {
            vector<int> ids;ids.reserve(candidates[nn_ranks[i]].ids.size());
            for(int p:candidates[nn_ranks[i]].ids)ids.push_back(nn106_index[p]);
            const float value=nn106.score(ids)[0];
            if(value>highest){highest=value;chosen=nn_ranks[i];}
        }
        LOCAL_NOTE(
            ++local_nn.calls;local_nn.scored+=n;local_nn.changed+=chosen!=nn_ranks[0];local_nn.rank_sum+=chosen;
            local_nn.feature_ms+=chrono::duration<double,milli>(encoded-started).count();
            local_nn.inference_ms+=chrono::duration<double,milli>(chrono::steady_clock::now()-encoded).count();
            local_nn.total_ms+=chrono::duration<double,milli>(chrono::steady_clock::now()-started).count();
        )
        return chosen;
    }
'''
    source=original
    start=source.index('namespace neural_rank {');end=source.index('\n#ifdef LOCAL',start)
    source=source[:start]+context+source[end:]
    start=source.index('// 未使用500件の5分割比較');end=source.index('class PortionRouter',start)
    source=source[:start]+source[end:]
    start=source.index('    array<double,32> nn_features(');end=source.index('    vector<int> nn_eligible;',start)
    source=source[:start]+source[end:]
    start=source.index('    int nn_choose(');end=source.index('    uint64_t alias_closures=',start)
    source=source[:start]+members+choose+'\n'+source[end:]
    source=source.replace('    void trajectory_changed() {','    void trajectory_changed() {\n        nn106_ready=false;nn106_items_ready=false;',1)
    source=source.replace('        const auto selector_values=nn_selector_features();\n        const bool use_nn=nn_selector_choice(selector_values);\n','')
    source=source.replace('const bool nn_eligible=use_nn&&!dependency&&mode>=3&&choice>=0;',
                          'const bool nn_eligible=!dependency&&mode>=3&&choice>=0;')
    source=source.replace('''                    const double progress=clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(LOCAL_SECONDS(0.02),end-lns_start),0.0,1.0);
                    choice=nn_choose(current,best,stagnant,progress,cooldown);''',
                          '                    choice=nn_choose(current,cooldown);')
    source=source.replace('nn_selector_choice(nn_selector_features())','1')
    source=source.replace('// 元の順位選択で適格候補がある場合だけ、同じ待ち条件でNNを使う。',
                          '// 元順位の適格候補を短縮量モデルで選ぶ。ランダム枠と依存拡張は保持する。')
    source='// '+output.name+'\n// v105の初期NNとLNSを保持し、通常候補の短縮量をv106で学習して選ぶ。\n'+source[source.index('#include'):]
    assert 'neural_rank::' not in source and 'nn_selector_choice' not in source and '#include "' not in source
    assert len(source.encode())<=512000,len(source.encode())
    output.write_text(source)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--model',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();build(a.model,a.output)
