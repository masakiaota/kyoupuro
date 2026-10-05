#!/usr/bin/env python3
"""全域と経路付近の再挿入を事前固定の割合で使い分ける。"""
from check_v113_integrated import block
from v089_data import ROOT, save, sha

RUN=ROOT/'results/nn_rank/v115/20261004_corridor_studio'
CONDITIONS={'r0':0,'r1':1}

STATS=r'''
struct CorridorStats {
    int64_t calls=0,completed=0,allowed_cells=0,total_cells=0,restricted_groups=0,full_groups=0;
    void summary() const {
        LOCAL_ONLY(
            trace.count_by("corridor_calls",calls);
            trace.count_by("corridor_completed",completed);
            trace.count_by("corridor_allowed_cells",allowed_cells);
            trace.count_by("corridor_total_cells",total_cells);
            trace.count_by("corridor_restricted_groups",restricted_groups);
            trace.count_by("corridor_full_groups",full_groups);
        );
    }
} corridor_stats;
'''

HISTORY=r'''
        // 個体が乗った便だけを列挙する。現在の操作列との世代一致は呼出し側で確認する。
        template<class Visit>
        void visit_flights(int token,Visit visit) const {
            for(int w=0;w<stride;++w) {
                uint64_t bits=rows[size_t(token)*stride+w].flight;
                while(bits) {
                    const int bit=__builtin_ctzll(bits);bits&=bits-1;
                    visit(64*w+bit);
                }
            }
        }
'''

METHOD=r'''
    bool corridor_enabled=false;
    array<uint8_t,max_cells> corridor_allowed{};
    void prepare_corridor(int token) {
        if(bit_history.generation!=trajectory_generation||history_generation!=trajectory_generation)
            throw logic_error("stale corridor history");
        corridor_allowed.fill(0);int count=0;
        auto add=[&](int p){if(p>=0&&!corridor_allowed[p]){corridor_allowed[p]=1;++count;}};
        auto around=[&](int p) {
            add(p);
            if constexpr(CORRIDOR_RADIUS==1)for(int d=0;d<4;++d)add(board_info.adj[p][d]);
        };
        around(token);around(board_info.nest_pos_by_code[board_info.initial[token]]);
        bit_history.visit_flights(token,[&](int t) {
            const Move m=current[t];around(m.p);around(board_info.ray[m.p][m.d][m.l]);
        });
        LOCAL_NOTE(corridor_stats.allowed_cells+=count;corridor_stats.total_cells+=board_info.cell_count;)
    }
'''

def build():
    RUN.mkdir(parents=True,exist_ok=True)
    parent=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
    assert sha(parent)=='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
    text=parent.read_text().replace('class TemporalLNS {',STATS+'\nclass TemporalLNS {',1)
    before=block(text,'        CandidateHistorySummary query(')
    text=text.replace(before,HISTORY+'\n'+before,1)
    before=block(text,'    bool insert_at(')
    after=before.replace('    bool insert_at(', '    template<bool limited>\n    bool insert_at_impl(',1)
    needle='                    int q=board_info.ray[p][d][l];\n                    if(h[q]>=8)continue;'
    assert after.count(needle)==1
    after=after.replace(needle,'                    int q=board_info.ray[p][d][l];\n'
                        '                    if constexpr(limited)if(!corridor_allowed[q])continue;\n'
                        '                    if(h[q]>=8)continue;',1)
    needle='                        const int dest=board_info.ray[tag_cell][d][l];\n                        if(h[dest]>=8)continue;'
    assert after.count(needle)==1
    after=after.replace(needle,'                        const int dest=board_info.ray[tag_cell][d][l];\n'
                        '                        if constexpr(limited)if(!corridor_allowed[dest])continue;\n'
                        '                        if(h[dest]>=8)continue;',1)
    after+='''
    bool insert_at(const State& initial,const vector<Move>& base,int source,int color_code,
                   int initial_gap,int cap,vector<Move>& result) {
        return insert_at_impl<false>(initial,base,source,color_code,initial_gap,cap,result);
    }
'''
    text=text.replace(before,METHOD+'\n'+after,1)
    before=block(text,'    bool insert(const State&')
    after=before.replace('        return insert_at(','''        if(corridor_enabled) {
            prepare_corridor(token);LOCAL_NOTE(++corridor_stats.calls;)
            const bool ok=insert_at_impl<true>(initial,base,token,int(board_info.initial[token]),0,cap,result);
            LOCAL_NOTE(corridor_stats.completed+=ok;)
            return ok;
        }
        return insert_at(''',1)
    text=text.replace(before,after,1)
    needle='                vector<pair<double,int>> order;order.reserve(cand.ids.size());'
    assert text.count(needle)==1
    text=text.replace(needle,'''                // 全域探索は成功・失敗に関係なく、反復番号だけで定期的に使う。
                corridor_enabled=iteration%4!=0;
                LOCAL_NOTE(if(corridor_enabled)++corridor_stats.restricted_groups;
                           else ++corridor_stats.full_groups;)
'''+needle,1)
    text=text.replace('        trace.count_by("floor_cells", board_info.cell_count);',
                      '        corridor_stats.summary();\n        trace.count_by("floor_cells", board_info.cell_count);',1)
    sources={}
    for label,radius in CONDITIONS.items():
        path=ROOT/f'adhoc/bin/v115_corridor_{label}.cpp'
        body='// '+path.name+'\n'+text.split('\n',1)[1].replace('CORRIDOR_RADIUS',str(radius))
        if path.exists():assert path.read_text()==body,'preserve prior source before repair'
        path.write_text(body)
        sources[label]=dict(path=str(path.relative_to(ROOT)),sha256=sha(path),radius=radius)
    save(RUN/'sources.json',dict(parent=sha(parent),sources=sources))

if __name__=='__main__':build()
