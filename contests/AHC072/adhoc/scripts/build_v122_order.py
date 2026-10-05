#!/usr/bin/env python3
"""v115の再挿入順に使う特徴と診断用の固定仕事量版を生成する。"""
import json
from check_v113_integrated import block
from v089_data import ROOT, save, sha, now

RUN = ROOT / 'results/nn_rank/v122/20261004_order_studio'
BASE = ROOT / 'src/bin/v115_corridor_reinsertion.cpp'

FEATURES = r'''
    struct OrderFeatures {
        vector<array<int,4>> orders;
        vector<array<float,80>> values;
    };
    OrderFeatures order_features(const vector<Move>& base,
                                const vector<pair<double,int>>& order,int allowance) const {
        if(history_generation!=trajectory_generation)throw logic_error("stale order history");
        const int n=int(order.size()),T=int(current.size());
        const float denominator=float(max(1,T));
        array<array<float,12>,4> token{};
        for(int a=0;a<n;++a) {
            const int id=order[a].second,code=board_info.initial[id];
            int first=T,last=0,used=0,rides=0,distance=0;
            bit_history.visit_flights(id,[&](int t) {
                first=min(first,t);last=max(last,t);++used;
                rides+=flights[t].n>1;distance+=current[t].l;
            });
            int closest=infinite_cost,sum=0,same=0;
            for(int b=0;b<n;++b)if(a!=b) {
                int other=order[b].second,d=board_info.floor_dist[id][other];
                closest=min(closest,d);sum+=d;same+=board_info.initial[other]==code;
            }
            token[a]={board_info.floor_dist[id][board_info.nest_pos_by_code[code]]/40.f,
                first/denominator,last/denominator,used/denominator,rides/denominator,
                bit_history.support_count(id)/denominator,a/float(n-1),closest/40.f,
                sum/(40.f*(n-1)),distance/(8.f*denominator),same/float(n-1),float(-order[a].first/40.)};
        }
        OrderFeatures out;array<int,4> perm{0,1,2,3};
        do {
            array<float,80> x{};
            x[0]=n/4.f;x[1]=board_info.M/256.f;x[2]=board_info.cell_count/400.f;x[3]=board_info.K/12.f;
            x[4]=T/800.f;x[5]=base.size()/800.f;x[6]=(allowance-int(base.size()))/40.f;x[7]=corridor_enabled;
            for(int a=0;a<n;++a)copy(token[perm[a]].begin(),token[perm[a]].end(),x.begin()+8+12*a);
            int at=56;
            for(int a=0;a<4;++a)for(int b=a+1;b<4;++b,at+=4)if(b<n) {
                int p=order[perm[a]].second,q=order[perm[b]].second;
                int pc=board_info.initial[p],qc=board_info.initial[q];
                x[at]=board_info.floor_dist[p][q]/40.f;
                x[at+1]=board_info.floor_dist[board_info.nest_pos_by_code[pc]][q]/40.f;
                x[at+2]=board_info.floor_dist[p][board_info.nest_pos_by_code[qc]]/40.f;
                x[at+3]=pc==qc;
            }
            out.orders.push_back(perm);out.values.push_back(x);
        }while(next_permutation(perm.begin(),perm.begin()+n));
        return out;
    }
'''


def core():
    assert sha(BASE) == '00c649abb06fa8f46927ba50e5f3b7f000c806c0029bbbec683e79712d04387e'
    text = BASE.read_text()
    anchor = '        CandidateHistorySummary query(const vector<int>& ids) const {'
    assert text.count(anchor) == 1
    accessor = '''        int support_count(int id) const {
            int total=0;
            for(int w=0;w<stride;++w)total+=__builtin_popcountll(rows[size_t(id)*stride+w].support);
            return total;
        }
'''
    text = text.replace(anchor, accessor + anchor, 1)
    anchor = '    bool insert_two_orders('
    assert text.count(anchor) == 1
    text = text.replace(anchor, FEATURES + '\n' + anchor, 1)
    return text


def build_collector():
    text = core()
    text = text.replace(block(text, 'double exact_elapsed_sec() const'),
                        'double exact_elapsed_sec() const { return 0.0; }', 1)
    loop = 'while(time_keeper.exact_elapsed_sec()<min(slice_end,end))'
    assert text.count(loop) == 1
    text = text.replace(loop, 'while(iteration<512)', 1)
    path = ROOT / 'adhoc/bin/v122_order_core.cpp'
    path.write_text('// ' + path.name + '\n' + text.split('\n', 1)[1])
    RUN.mkdir(parents=True, exist_ok=True)
    save(RUN / 'collector_source.json', dict(parent_sha256=sha(BASE), core_sha256=sha(path), created_at=now()))


def model_cpp(parameters):
    def value(x):
        if isinstance(x,list):return '{'+','.join(value(y) for y in x)+'}'
        return float(x).hex()+'f'
    return '''namespace order_policy {
static constexpr float first[16][80]='''+value(parameters['0.weight'])+''';
static constexpr float bias[16]='''+value(parameters['0.bias'])+''';
static constexpr float last[16]='''+value(parameters['2.weight'][0])+''';
static constexpr float intercept='''+value(parameters['2.bias'][0])+''';
float predict(const array<float,80>& input) {
    float answer=intercept;
    for(int j=0;j<16;++j) {
        float z=bias[j];for(int k=0;k<80;++k)z+=first[j][k]*input[k];
        answer+=last[j]*max(0.f,z);
    }
    return answer;
}
}
'''


def build_submission():
    report=json.loads((RUN/'training/result.json').read_text())
    assert report['development']['passed']
    model=json.loads((RUN/'training/model.json').read_text())
    assert model['checkpoint_sha256']==sha(RUN/'training/latest.pt')
    text=core()
    text=text.replace('class TemporalLNS {',model_cpp(model['parameters'])+'\nclass TemporalLNS {',1)
    original=block(text,'bool insert_two_orders(')
    changed=original.replace('        int best_length_value=allowance+1;',r'''
        array<int,4> alternate{1,0,2,3};
        if(n>=3) {
            LOCAL_ONLY(const auto order_begin=chrono::steady_clock::now();)
            const auto context=order_features(base,order,allowance);
            float best_score=numeric_limits<float>::infinity();int selected=-1;
            // 第1順序は保持する。残る全順列から実際に試す第2順序を一つだけ選ぶ。
            for(int i=1;i<int(context.values.size());++i) {
                const float score=order_policy::predict(context.values[i]);
                if(!isfinite(score))throw logic_error("nonfinite order prediction");
                if(score<best_score){best_score=score;selected=i;}
            }
            if(selected<0)throw logic_error("missing alternate order");
            const auto chosen=context.orders[selected];
            LOCAL_ONLY(trace.count_by("order_nn_calls",1);
                       trace.count_by("order_nn_scored",context.values.size()-1);
                       trace.count_by("order_nn_changed",chosen!=alternate);
                       trace.add_time_ms("order_nn",chrono::duration<double,milli>(chrono::steady_clock::now()-order_begin).count());)
            alternate=chosen;
        }
        int best_length_value=allowance+1;''',1)
    changed=changed.replace('const int at=branch&&j<2?1-j:j,id=order[at].second;',
                            'const int at=branch?alternate[j]:j,id=order[at].second;',1)
    assert changed!=original and 'branch&&j<2' not in changed
    text=text.replace(original,changed,1)
    source=ROOT/'adhoc/bin/v122_learned_order.cpp'
    source.write_text('// '+source.name+'\n'+text.split('\n',1)[1])
    save(RUN/'sources.json',dict(sources={'learned':dict(path=str(source.relative_to(ROOT)),sha256=sha(source))},
                                 parent_sha256=sha(BASE),model_sha256=sha(RUN/'training/model.json'),created_at=now()))
    return source


if __name__ == '__main__':
    build_collector()
