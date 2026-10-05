#!/usr/bin/env python3
"""v113へ一時支持を仮定した二匹の再挿入だけを追加する。"""
from pathlib import Path
from check_v113_integrated import block
from v089_data import ROOT, save, sha

RUN = ROOT/'results/nn_rank/v114/20261004_cooperative_studio'
CONDITIONS = {'b08': .08, 'b20': .20}

STATS = r'''
struct CooperativeInsertStats {
    long long calls=0,static_ok=0,first_ok=0,demands=0,shadow_ok=0;
    long long pair_ok=0,completed=0,selected=0,raw_saved=0;
    double seconds=0;
    void summary() const {
        LOCAL_ONLY(
            trace.count_by("cooperative_calls",calls);
            trace.count_by("cooperative_static_ok",static_ok);
            trace.count_by("cooperative_first_ok",first_ok);
            trace.count_by("cooperative_demands",demands);
            trace.count_by("cooperative_shadow_ok",shadow_ok);
            trace.count_by("cooperative_pair_ok",pair_ok);
            trace.count_by("cooperative_completed",completed);
            trace.count_by("cooperative_selected",selected);
            trace.count_by("cooperative_raw_saved",raw_saved);
            trace.add_time_ms("cooperative",seconds*1000.0);
        );
    }
} cooperative_insert_stats;
'''

METHODS = r'''
    double cooperative_seconds=0;
    // 後で戻す個体を底に残す。仮の背景も、この固定個体を含めれば全操作が合法である。
    bool static_support_schedule(const State& initial,const vector<Move>& base,
                                 int support,vector<Move>& fixed) {
        if(initial[support]||board_info.nest_code[support])return false;
        State board=initial;board[support]=board_info.initial[support];
        fixed.clear();fixed.reserve(base.size());
        for(Move m:base) {
            if(m.p==support)++m.k;
            const int q=board_info.ray[m.p][m.d][m.l];
            if(q<0||m.k>=height(board[m.p])||m.l>m.k+1||
               height(board[q])+height(board[m.p])-m.k>8)return false;
            board_info.apply(board,m);fixed.push_back(m);
        }
        if(board[support]!=board_info.initial[support]||board_info.count(board)!=1)
            throw logic_error("static support was moved");
        return true;
    }

    // 固定個体を取り除いた影の予定。支持不足の跳躍は、次の条件付きDPが実現するまで出力しない。
    bool remove_static_support(const State& initial,const vector<Move>& fixed,
                               int support,vector<Move>& shadow,int& last,int& demands) {
        State board=initial;shadow.clear();last=-1;demands=0;
        for(Move m:fixed) {
            if(m.p==support) {
                if(m.k==0)throw logic_error("fixed support departed");
                --m.k;
            }
            const int p=m.p,q=board_info.ray[p][m.d][m.l];
            if(q<0||m.k>=height(board[p])||m.l>m.k+2||
               height(board[q])+height(board[p])-m.k>8)return false;
            if(m.l>m.k+1){last=int(shadow.size());++demands;}
            const TowerBits flight=reverse_tower(board[p]>>(4*m.k));
            board[p]=board_info.normalize(prefix(board[p],m.k),p);
            board[q]=board_info.normalize(append_tower(board[q],flight),q);
            shadow.push_back(m);
        }
        if(board_info.count(board))throw logic_error("shadow leaves background slimes");
        return demands>0;
    }

    bool insert_cooperative_order(const State& initial,const vector<Move>& base,
                                  const vector<pair<double,int>>& order,int role,int allowance,
                                  vector<Move>& result) {
        auto& stats=cooperative_insert_stats;++stats.calls;
        const double started=time_keeper.exact_elapsed_sec();
        struct Measure {
            double started;double& total;
            ~Measure(){const double dt=time_keeper.exact_elapsed_sec()-started;
                total+=dt;cooperative_insert_stats.seconds+=dt;}
        } measure{started,cooperative_seconds};
        const int first=order[role].second,support=order[1-role].second;
        vector<Move> fixed,with_first,shadow,tail;
        if(!static_support_schedule(initial,base,support,fixed))return false;
        ++stats.static_ok;
        State restored=initial;restored[support]=board_info.initial[support];
        if(!insert(restored,fixed,first,allowance-int(fixed.size()),with_first))return false;
        ++stats.first_ok;
        restored=initial;restored[first]=board_info.initial[first];
        int last,demands;
        if(!remove_static_support(restored,with_first,support,shadow,last,demands))return false;
        ++stats.shadow_ok;stats.demands+=demands;
        if(!insert_at_impl<true>(restored,shadow,support,int(board_info.initial[support]),0,
                                allowance-int(shadow.size()),tail,last))return false;
        ++stats.pair_ok;restored[support]=board_info.initial[support];
        for(size_t j=2;j<order.size();++j) {
            const int id=order[j].second;vector<Move> next;
            if(!insert(restored,tail,id,allowance-int(tail.size()),next))return false;
            tail=move(next);restored[id]=board_info.initial[id];
        }
        ++stats.completed;result=move(tail);return true;
    }
'''

def build():
    RUN.mkdir(parents=True, exist_ok=True)
    parent=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
    original=parent.read_text()
    assert sha(parent)=='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
    conditional=(ROOT/'src/bin/v314_conditional_support.cpp').read_text()
    text=original.replace('class TemporalLNS {',STATS+'\nclass TemporalLNS {',1)
    old=block(text,'    bool insert_at(')
    templated=block(conditional,'    template<bool conditioned>\n    bool insert_at_impl(')
    wrapper=block(conditional,'    bool insert_at(')
    text=text.replace(old,templated+'\n\n'+wrapper,1)
    text=text.replace('    // 第2順序では先頭2匹だけを交換する。',METHODS+'\n    // 第2順序では先頭2匹だけを交換する。',1)
    before=block(text,'    bool insert_two_orders(')
    after=before.replace('vector<Move>& result) {','vector<Move>& result,bool cooperate=false) {',1)
    after=after.replace('        const bool ok=best_length_value<=allowance;',r'''
        if(cooperate) {
            vector<Move> candidate;
            const int bound=min(allowance,best_length_value-1);
            if(insert_cooperative_order(initial,base,order,iteration&1,bound,candidate)) {
                ++cooperative_insert_stats.selected;
                if(best_length_value<=allowance)
                    cooperative_insert_stats.raw_saved+=best_length_value-int(candidate.size());
                best_length_value=int(candidate.size());result=move(candidate);
            }
        }
        const bool ok=best_length_value<=allowance;''',1)
    text=text.replace(before,after,1)
    text=text.replace('ok=insert_two_orders(initial,base,order,int(current.size())+4,rebuilt);',
                      'ok=insert_two_orders(initial,base,order,int(current.size())+4,rebuilt,\n'
                      '                        cooperative_seconds < COOPERATIVE_FRACTION *\n'
                      '                        (active_seconds+time_keeper.exact_elapsed_sec()-slice_begin));',1)
    text=text.replace('        trace.count_by("floor_cells", board_info.cell_count);',
                      '        cooperative_insert_stats.summary();\n        trace.count_by("floor_cells", board_info.cell_count);',1)
    sources={}
    for label,fraction in CONDITIONS.items():
        path=ROOT/f'adhoc/bin/v114_cooperative_{label}.cpp'
        body='// '+path.name+'\n'+text.split('\n',1)[1].replace('COOPERATIVE_FRACTION',str(fraction))
        if path.exists() and path.read_text()!=body:raise RuntimeError('preserve old source before repair')
        path.write_text(body)
        sources[label]={'path':str(path.relative_to(ROOT)),'sha256':sha(path),'fraction':fraction}
    save(RUN/'sources.json',{'parent':sha(parent),'sources':sources})
    return sources

if __name__=='__main__':build()
