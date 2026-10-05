from pathlib import Path
from v089_data import sha
ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'results/nn_rank/v136/20261005_exact_speed_studio'
BASE=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
SOURCE=ROOT/'src/bin/v136_exact_speed.cpp'
BASE_SHA='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'

def build():
    assert sha(BASE)==BASE_SHA
    s=BASE.read_text().replace('// v113_integrated_nn_lns.cpp','// v136_exact_speed.cpp',1)
    s=s.replace('// v113: v111 policy and incremental CNN; fixed integration HR.',
                '// v136: exact-operation speedups of v113; unchanged NN weights and search budgets.',1)
    def replace(a,b,n=1):
        nonlocal s
        assert s.count(a)==n,(a[:100],s.count(a),n)
        s=s.replace(a,b)
    replace('#pragma GCC optimize("O3,unroll-loops")', '#pragma GCC optimize("O3,unroll-loops")\n#if defined(__x86_64__) && !defined(AHC072_PORTABLE)\n#pragma GCC target("avx2,bmi,bmi2,lzcnt,popcnt")\n#endif')
    start=s.index('array<array<float, 40>, 400> board_features(')
    end=s.index('\nvoid read_snapshot',start)
    before=s[start:end]
    inner=before[before.index('        const auto [i, j]'):before.index('\n    }\n    return result;')]
    inner=inner.replace('        auto& x = result[i * 20 + j];','        array<float,40> x{};')
    s=s[:start]+'''array<float,40> cell_features(const Input& input,const Board& board,const BoardState& state,int p) {
'''+inner+'''
    return x;
}
#ifdef V090_DIAGNOSTIC
array<array<float,40>,400> board_features(const Input& input,const Board& board,const BoardState& state) {
    array<array<float,40>,400> result{};
    for(int p=0;p<board.cell_count;++p)result[flat_cell(board,p)]=cell_features(input,board,state,p);
    return result;
}
#endif
'''+s[end:]
    # The optional owner keeps the diagnostic's one-shot cache alive. Production
    # borrows its search-owned cache and never copies the 400x48 final layer.
    a=s.index('struct Encoded {');b=s.index('// The cache belongs',a)
    s=s[:a]+s[b:]
    replace('    bool valid=false;\n    void invalidate()', '    bool valid=false;\n    array<array<int16_t,9>,400> neighbors{};\n    bool geometry_ready=false;\n    void invalidate()')
    replace('#ifdef LOCAL\nstruct IncrementalStats {','''struct Encoded {
    const array<array<float,NN_WIDTH>,400>* cells=nullptr;
    array<float,NN_WIDTH> global{};
    unique_ptr<EncodeCache> owner;
    float remaining=0.f;
};
#ifdef LOCAL
struct IncrementalStats {''')
    replace('    auto& cache=*retained;','''    auto& cache=*retained;
    if(!cache.geometry_ready) {
        for(int p=0;p<board.cell_count;++p) {
            const auto [i,j]=board.coordinates[p];
            for(int di=-1;di<=1;++di)for(int dj=-1;dj<=1;++dj) {
                const int ni=i+di,nj=j+dj;
                cache.neighbors[p][(di+1)*3+dj+1]=
                    (ni<0||ni>=input.N||nj<0||nj>=input.N)?-1:board.cell_id[ni][nj];
            }
        }
        cache.geometry_ready=true;
    }''')
    replace('    const auto features=board_features(input,board,state);\n    auto& first=', '    auto& first=')
    replace('        channel_matvec(fast_input,features[flat_cell(board,p)].data(),first[p].data());',
            '        const auto features=cell_features(input,board,state,p);\n        channel_matvec(fast_input,features.data(),first[p].data());')
    a=s.index('        // Propagate the exact graph dependency');b=s.index('        changed=affected;',a)
    s=s[:a]+'''        // Geometry is constant. Each output retains the parent's nine-neighbor
        // accumulation order; its pointwise operation needs only this depth row.
        for(int p=0;p<board.cell_count;++p) {
            affected[p]=changed[p];
            for(int q:cache.neighbors[p])if(q>=0 && changed[q])affected[p]=true;
        }
        for(int p=0;p<board.cell_count;++p)if(affected[p]) {
            auto depth=nn_depth_b[block];
            for(int k=0;k<9;++k) {
                const int q=cache.neighbors[p][k];if(q<0)continue;
                const auto& weights=fast_depth[block][k];
                for(int c=0;c<NN_WIDTH;++c)depth[c]+=weights[c]*h[q][c];
            }
            for(float& v:depth)v=max(0.f,v);
            auto residual=nn_point_b[block];
            channel_matvec(fast_point[block],depth.data(),residual.data());
            for(int o=0;o<NN_WIDTH;++o)output[p][o]=max(0.f,h[p][o]+residual[o]);
#ifdef LOCAL
            ++incremental_stats.cells[block+1];
#endif
        }
'''+s[b:]
    replace('    result->cells=cache.layers[NN_DEPTH];\n    const auto& h=result->cells;',
            '    result->cells=&cache.layers[NN_DEPTH];\n    result->owner=move(fresh);\n    const auto& h=*result->cells;')
    replace('    float raw=nn_critic_out_b[0];','    // Greedy policy never reads the value head. Keep it for numeric diagnostics.\n#ifdef V090_DIAGNOSTIC\n    float raw=nn_critic_out_b[0];')
    replace('    result->remaining=(raw>20.f?raw:log1p(exp(raw)))*100.f;','    result->remaining=(raw>20.f?raw:log1p(exp(raw)))*100.f;\n#endif')
    replace('encoded.cells[p].data()', '(*encoded.cells)[p].data()',2)
    a=s.index('        vector<int> order(moves.size());iota(order.begin(),order.end(),0);')
    b=s.index('        if(chosen<0){result.exhausted=true;break;}',a)
    s=s[:a]+'''        // Usually the highest logit is usable. Sorting is needed only when
        // that successor was visited; ties still follow the legal-move order.
        const int best=int(max_element(logits.begin(),logits.end())-logits.begin());
        next=state;next.apply(board,moves[best]);
        if(!visited.contains(next.bits)) {
            chosen=best;visited.insert(next.bits);
        }else {
            ++result.duplicates;
            vector<int> order(moves.size());iota(order.begin(),order.end(),0);
            stable_sort(order.begin(),order.end(),[&](int a,int b){return logits[a]>logits[b];});
            for(size_t rank=1;rank<order.size();++rank) {
                const int index=order[rank];
                next=state;next.apply(board,moves[index]);
                if(visited.contains(next.bits)){++result.duplicates;continue;}
                chosen=index;visited.insert(next.bits);break;
            }
        }
'''+s[b:]
    marker='    int count(const State &b) const {'
    helper='''    // Only the validated background plan of reinsertion uses this entry.
    // Its cached heights include homecoming after the preceding operation.
    // General moves still go through apply() and its legality checks.
    void apply_background(State& b,Move m,int q,int& hp,int& hq) const {
#ifdef V136_CHECK_BACKGROUND
        if(hp!=height(b[m.p])||hq!=height(b[q]))throw logic_error("cached height mismatch");
        const TowerBits saved_p=b[m.p],saved_q=b[q];
        apply(b,m);const TowerBits expected_p=b[m.p],expected_q=b[q];
        b[m.p]=saved_p;b[q]=saved_q;
#endif
        const int n=hp-m.k;
        TowerBits flight=b[m.p]>>(4*m.k);
        flight=((flight>>4)&0x0f0f0f0fu)|((flight&0x0f0f0f0fu)<<4);
        flight=__builtin_bswap32(flight)>>((8-n)*4);
        TowerBits left=prefix(b[m.p],m.k),land=b[q]|(flight<<(4*hq));
        hp=m.k;hq+=n;
        auto home=[&](TowerBits a,int cell,int& h) {
            const int code=nest_code[cell];
            if(code && h) {
                h=height((a^(0x11111111u*code))&tower_mask[h]);
                a=prefix(a,h);
            }
            return a;
        };
        b[m.p]=home(left,m.p,hp);b[q]=home(land,q,hq);
#ifdef V136_CHECK_BACKGROUND
        if(b[m.p]!=expected_p||b[q]!=expected_q||hp!=height(b[m.p])||hq!=height(b[q]))throw logic_error("background replay mismatch");
        ++background_audited;
#endif
    }
#ifdef V136_CHECK_BACKGROUND
    static inline uint64_t background_audited=0;
#endif
'''
    replace(marker,helper+marker)
    replace('board_info.apply(b,m);h[p]=height(b[p]);h[q]=height(b[q]);',
            'board_info.apply_background(b,m,q,h[p],h[q]);',2)
    if SOURCE.exists():assert SOURCE.read_text()==s
    else:SOURCE.write_text(s)
    return SOURCE
if __name__=='__main__':print(build())
