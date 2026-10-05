#!/usr/bin/env python3
"""v213のソースを作る。solverは実行しない。"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'src/bin/v213_atomic_color_runs.cpp'
parent = (ROOT / 'src/bin/v210_collection_color_quotient.cpp').read_text()
s = parent


def replace(old, new, count=1):
    global s
    assert s.count(old) == count, (old[:100], s.count(old), count)
    s = s.replace(old, new)


def section(start, stop, new):
    global s
    a, b = s.index(start), s.index(stop, s.index(start))
    s = s[:a] + new + s[b:]


s = ('// v213_atomic_color_runs.cpp\n'
     '// v213: 同色の連続部分を全工程で分割しない。塔は色と匹数の塊を64 bitに詰める。\n'
     '// v210の時間・選抜条件を維持し、構築・LNS・経路短縮に同じ制約を適用する。\n'
     + s[s.index('// 探索の実数演算'):])
replace('using TowerBits = uint32_t;', 'using TowerBits = uint64_t;')
replace('// 塔は下から4 bitずつ色c+1を詰める。空は0。色符号は1..K、巣なしは0。',
        '// 下から1 byteずつ同色の塊を詰める。下位4 bitは色c+1、上位4 bitは匹数−1。')
section('constexpr TowerBits tower_mask', '// 単位費用の探索用。', r'''// 同色8匹も1 byteで表し、反転はbyte交換、分割候補は塊の境界だけを巡回する。
// 塔全体を固定長にして、盤面コピーとハッシュのための動的確保を避ける。
inline int runs(TowerBits a) { return a ? (64-__builtin_clzll(a)+7)/8 : 0; }
inline int height(TowerBits a) {
    const uint64_t extras=(a>>4)&0x0f0f0f0f0f0f0f0fULL;
    return runs(a)+int((extras*0x0101010101010101ULL)>>56);
}
inline int first_run_size(TowerBits a) { return int((a>>4)&15)+1; }
inline int top_color_code(TowerBits a) { return a ? int((a>>(8*(runs(a)-1)))&15) : 0; }
inline int top_run_size(TowerBits a) { return a ? int((a>>(8*(runs(a)-1)+4))&15)+1 : 0; }
inline int mono_color_code(TowerBits a) { return !a ? 0 : (a<256 ? int(a&15) : -1); }
inline TowerBits reverse_tower(TowerBits a) {
    return a ? __builtin_bswap64(a)>>(8*(8-runs(a))) : 0;
}
inline TowerBits append_tower(TowerBits lower,TowerBits upper) {
    if(!lower)return upper;
    if(!upper)return lower;
    const int shift=8*(runs(lower)-1);
    if(int((lower>>shift)&15)==int(upper&15)) {
        // 接合した同色の塊を統合し、以後の分割候補から内部を消す。
        lower+=TowerBits(first_run_size(upper))<<(shift+4);
        upper>>=8;
    }
    const int offset=8*runs(lower);
    assert(!upper||offset<64);
    return upper ? lower|(upper<<offset) : lower;
}
// 仮想的な除去・逆探索にも使うため、切出し自体は塊の途中を扱える。
// 実際の順方向操作を作る箇所は必ずrun_boundaryで検査する。
inline TowerBits prefix(TowerBits a,int k) {
    TowerBits result=0;int shift=0;
    while(a&&k>0) {
        const int n=min(k,first_run_size(a));
        result|=TowerBits((a&15)|((n-1)<<4))<<shift;
        k-=n;a>>=8;shift+=8;
    }
    return result;
}
inline TowerBits suffix(TowerBits a,int k) {
    while(a&&k>=first_run_size(a)){k-=first_run_size(a);a>>=8;}
    if(k&&a)a-=TowerBits(k)<<4;
    return a;
}
inline int color_at(TowerBits a,int k) { return int(suffix(a,k)&15); }
inline bool run_boundary(TowerBits a,int k) {
    while(k>0&&a){k-=first_run_size(a);a>>=8;}
    return k==0;
}
struct RunCuts {
    struct Iterator {
        TowerBits rest;int keep;
        int operator*()const{return keep;}
        Iterator& operator++(){keep+=first_run_size(rest);rest>>=8;return *this;}
        bool operator!=(const Iterator& other)const{return rest!=other.rest;}
    };
    TowerBits word;
    Iterator begin()const{return {word,0};}
    Iterator end()const{return {0,0};}
};
inline TowerBits insert_word(TowerBits a,int gap,TowerBits piece) {
    return append_tower(append_tower(prefix(a,gap),piece),suffix(a,gap));
}
// 有限区間の索引は従来の32 bit色列を使い、位置と合わせて64 bitに保つ。
inline uint32_t expanded_word(TowerBits a) {
    uint32_t word=0;int shift=0;
    for(;a;a>>=8)for(int i=0;i<first_run_size(a);i++,shift+=4)word|=uint32_t(a&15)<<shift;
    return word;
}
inline TowerBits packed_word(uint32_t a) {
    TowerBits word=0;for(;a;a>>=4)word=append_tower(word,TowerBits(a&15));return word;
}
struct InfeasibleConstruction {};
#ifdef LOCAL
struct AtomicStats {
    int64_t dp_positions=0,dp_boundaries=0,router_join_stops=0;
    int64_t insert_split_prunes=0,extract_split_prunes=0,run_reinsertions=0,run_reinserted_slimes=0;
    int64_t board_operations=0,departure_runs=0,departure_slimes=0,blocked_constructions=0;
    void summary()const {
        auto add=[&](const char* key,int64_t n){trace.count_by(string("atomic_")+key,n);};
        add("dp_positions",dp_positions);add("dp_boundaries",dp_boundaries);
        add("router_join_stops",router_join_stops);add("insert_split_prunes",insert_split_prunes);
        add("extract_split_prunes",extract_split_prunes);add("run_reinsertions",run_reinsertions);
        add("run_reinserted_slimes",run_reinserted_slimes);add("board_operations",board_operations);
        add("departure_runs",departure_runs);add("departure_slimes",departure_slimes);
        add("blocked_constructions",blocked_constructions);add("tower_bytes",sizeof(TowerBits));
    }
} atomic_stats;
#endif

''')
section('inline int color_mask(TowerBits a)', '// pは床ID', r'''inline int color_mask(TowerBits a) {
    int m=0;for(;a;a>>=8)m|=1<<((a&15)-1);return m;
}

''')
# 個体段数による右シフトは塊列の切出しへ置き換える。
expressions = {
    'b[p]>>(4*m.k)': 'suffix(b[p],m.k)',
    's[o]>>(4*k)': 'suffix(s[o],k)',
    'word>>(4*a.k)': 'suffix(word,a.k)',
    'b[s]>>(4*k)': 'suffix(b[s],k)',
    'b[p]>>(4*k)': 'suffix(b[p],k)',
    'end[q]>>(4*m.k)': 'suffix(end[q],m.k)',
    's[a]>>(4*k)': 'suffix(s[a],k)',
    's[b]>>(4*(hb-amount))': 'suffix(s[b],hb-amount)',
    'w.end[a]>>(4*m.k)': 'suffix(w.end[a],m.k)',
    'check[a]>>(4*m.k)': 'suffix(check[a],m.k)',
    'board[p]>>(4*first.k)': 'suffix(board[p],first.k)',
    'end[p]>>(4*m.k)': 'suffix(end[p],m.k)',
    'end[r]>>(4*m.k)': 'suffix(end[r],m.k)',
    'board[p]>>(4*keep)': 'suffix(board[p],keep)',
    'board[first.p]>>(4*first.k)': 'suffix(board[first.p],first.k)',
    'board[goal]>>(4*m.k)': 'suffix(board[goal],m.k)',
    'b[p]>>(4*h)': 'suffix(b[p],h)',
    'w>>(4*matched)': 'suffix(w,matched)',
    'board[p]>>(4*keep)': 'suffix(board[p],keep)',
    'before[p]>>(4*keep)': 'suffix(before[p],keep)',
    'b[a.p]>>(4*a.k)': 'suffix(b[a.p],a.k)',
    'before>>(4*cut.keep)': 'suffix(before,cut.keep)',
    'boundary[p]>>(4*keep_p)': 'suffix(boundary[p],keep_p)',
    'boundary[q]>>(4*k)': 'suffix(boundary[q],k)',
    'b[m.p]>>(4*m.k)': 'suffix(b[m.p],m.k)',
}
for old,new in expressions.items():
    assert old in s, old
    s=s.replace(old,new)
replace('''        TowerBits different=(a^(0x11111111u*code))&tower_mask[height(a)];
        return prefix(a,height(different));''', '''        if(top_color_code(a)!=code)return a;
        const int shift=8*(runs(a)-1);
        return shift ? a&((TowerBits(1)<<shift)-1) : 0;''')
replace('''        int h=height(b[p]);
        if(m.k>=h''', '''        int h=height(b[p]);
        if(!run_boundary(b[p],m.k))throw logic_error("atomic color split");
        LOCAL_ONLY(++atomic_stats.board_operations;atomic_stats.departure_runs+=runs(b[p]);atomic_stats.departure_slimes+=h);
        if(m.k>=h''')
# EmptyDPはもともと同色の境界を切らない。塊列から直接列挙する。
replace('''            for(int k=1;k<h;k++) {
                if(((s[o]>>(4*(k-1)))&15)==((suffix(s[o],k))&15)) continue;''', '''            for(int k:RunCuts{s[o]}) {if(k==0)continue;''')
# DeliveryDPは背景の色も保持し、接合した荷物だけを再出発させない。
replace('''    int bh[max_cells];
    vector<Arc> incoming''', '''    int bh[max_cells],background_top[max_cells];
    vector<Arc> incoming''')
replace('''            for(int k=1;k<h;k++) {
                left[o][k]''', '''            LOCAL_ONLY(atomic_stats.dp_positions+=max(0,h-1);atomic_stats.dp_boundaries+=runs(s[o])-1);
            for(int k:RunCuts{s[o]}) {if(k==0)continue;
                left[o][k]''')
replace('''            } else for(int k=1;k<h;k++) {
                int base=1+value(left[o][k],p);''', '''            } else for(int k:RunCuts{s[o]}) {if(k==0)continue;
                int base=1+value(left[o][k],p);''')
replace('''                if(bh[q]+h>8||board_info.nest_code[q]==tc[no]) continue;''', '''                if(bh[q]+h>8||board_info.nest_code[q]==tc[no]) continue;
                if(background_top[q]==int(s[no]&15)){LOCAL_ONLY(++atomic_stats.router_join_stops);continue;}''')
replace('''        for(int p=0;p<board_info.cell_count;p++) bh[p]=p==selected ? 0 : height(b[p]);''', '''        for(int p=0;p<board_info.cell_count;p++) {
            bh[p]=p==selected ? 0 : height(b[p]);
            background_top[p]=p==selected ? 0 : top_color_code(b[p]);
        }''')
replace('throw logic_error("unreachable delivery")', 'throw InfeasibleConstruction()')
replace('throw logic_error("unreachable event")', 'throw InfeasibleConstruction()')
# Router: 全塔を運ぶ。途中の同色接合は終点としてだけ許す。
replace('''            if(p!=src&&board_info.nest_code[p]==tc[o]) continue;''', '''            if(p!=src&&board_info.nest_code[p]==tc[o]) continue;
            if(p!=src&&top_color_code(b[p])==tc[o^1]){LOCAL_ONLY(++atomic_stats.router_join_stops);continue;}''')
# PortionRouter/FiniteRouterの出発と中継に同じ規則を適用する。
replace('''        fill(ds,ds+2*board_info.cell_count,int16_t(-1));
        int que''', '''        fill(ds,ds+2*board_info.cell_count,int16_t(-1));
        if(k>=height(b[s])||!run_boundary(b[s],k))return;
        const int source_top=top_color_code(board_info.normalize(prefix(b[s],k),s));
        int que''')
replace('''            if(z!=s*2&&board_info.nest_code[p]==top[o]) continue;''', '''            if(z!=s*2&&board_info.nest_code[p]==top[o]) continue;
            const int lower_top=p==s?source_top:top_color_code(b[p]);
            if(z!=s*2&&lower_top==top[o^1]){LOCAL_ONLY(++atomic_stats.router_join_stops);continue;}''')
replace('''        fill(distance,distance+2*board_info.cell_count,int16_t(-1));
        int queue''', '''        fill(distance,distance+2*board_info.cell_count,int16_t(-1));
        if(k>=height(b[p])||!run_boundary(b[p],k))return;
        const int source_top=top_color_code(board_info.normalize(prefix(b[p],k),p));
        int queue''')
replace('''            if(distance[at]>=cap||(at!=p*2&&board_info.nest_code[v]==top[o]))continue;''', '''            if(distance[at]>=cap||(at!=p*2&&board_info.nest_code[v]==top[o]))continue;
            const int lower_top=v==p?source_top:top_color_code(b[v]);
            if(at!=p*2&&lower_top==top[o^1]){LOCAL_ONLY(++atomic_stats.router_join_stops);continue;}''')
# 基礎集荷木では他色の1匹が残る通過床にも全塊が載るよう、巣以外への集荷を7匹までにする。
section('''        auto top_count=[&](int p) {''', '''        function<void(int)> flush''', '''        auto top_count=[&](int p) {
            return top_color_code(w.state[p])==current_code?top_run_size(w.state[p]):0;
        };
''')
replace('''                int amt=top_count(p),space=8-height(w.state[v]);
                if(space<amt&&top_count(v)) { flush(v);space=8-height(w.state[v]); }
                int take=min(amt,space);
                if(take<=0) throw logic_error("baseline capacity");''', '''                const int amt=top_count(p);
                auto capacity=[&]{return min(8-height(w.state[v]),(v==root?8:7)-top_count(v));};
                if(capacity()<amt&&top_count(v))flush(v);
                const int take=amt;
                if(capacity()<take)throw logic_error("atomic baseline capacity");''')
replace('''                    if(q<0||board_info.adj[dest][d]!=q||height(w.state[q])+take>8) break;''', '''                    if(q<0||board_info.adj[dest][d]!=q||height(w.state[q])+take>8||
                       (q!=root&&top_count(q)+take>7)) break;''')
# 期限付近も最上段の塊全体を帰す。移動中に同色背景へ接合する床は通過しない。
replace('期限付近の1匹ずつの帰巣。', '期限付近の同色の塊ごとの帰巣。')
s=s.replace('finish_singles','finish_runs')
replace('''        bh[s]=height(board_info.normalize(prefix(w.state[s],hs-1),s));''', '''        const int amount=top_run_size(w.state[s]),leave=hs-amount;
        bh[s]=height(board_info.normalize(prefix(w.state[s],leave),s));''')
replace('''                if(bh[v]>=8||prev[v]!=-2) continue;''', '''                if(bh[v]+amount>8||prev[v]!=-2)continue;
                if(v!=goal&&top_color_code(w.state[v])==current_code)continue;''')
replace('throw logic_error("single blocked by full towers")','throw InfeasibleConstruction()')
replace('uint8_t(p==s?hs-1:bh[p])','uint8_t(p==s?leave:bh[p])')
# 対象外の同色塔に接合すると、元の背景を残す集荷木では再出発できない。
replace('''                    if(background[next]+candidate.pieces>8)continue;''', '''                    if(background[next]+candidate.pieces>8)continue;
                    if(background[next]&&top_color_code(w.state[next])==color)continue;''')
# 前向き共同探索は境界だけ。逆探索は直前状態の順方向境界を検査する。
replace('''                for(int k=max(l-1,ha+hb-8);k<ha;k++) {''', '''                for(int k:RunCuts{s[a]}) {
                    if(k<max(l-1,ha+hb-8))continue;''')
replace('''                    previous[b]=prefix(s[b],hb-amount);
                    if(!emit''', '''                    previous[b]=prefix(s[b],hb-amount);
                    if(!run_boundary(previous[a],ha))continue;
                    if(!emit''')
# 同色を先発・後発に割る専用短縮は制約下で発動しないため削除する。
section('#ifdef LOCAL\nstruct MonoDispatchStats', 'struct CoupledRoutesStats', 'struct CoupledRoutesStats')
replace('struct CoupledRoutesStatsstruct CoupledRoutesStats','struct CoupledRoutesStats')
replace('FINAL_JOINT,FINAL_MONO,FINAL_STRICT','FINAL_JOINT,FINAL_STRICT')
replace('"joint","mono","strict"','"joint","strict"')
replace('local_mono_dispatch.summary();coupled_routes.summary();','coupled_routes.summary();')
replace('''                check();apply_final_reduction(answer,FINAL_MONO,stats,[&]{return compress_mono_dispatch(answer,deadline);});
''','')
replace('''            if(time_keeper.exact_elapsed_sec()<deadline)
                apply(answer,FINAL_MONO,[&]{return compress_mono_dispatch(answer,deadline);});
''','')
# 有限区間の索引だけは32 bit色列へ変換して従来のコンパクトなキーを保つ。
replace('static TowerBits word(uint64_t x){return TowerBits(x);}', 'static TowerBits word(uint64_t x){return packed_word(uint32_t(x));}')
replace('(uint64_t(p)<<32)|w','(uint64_t(p)<<32)|expanded_word(w)')
replace('(uint64_t(p)<<32)|goal_word[p]','(uint64_t(p)<<32)|expanded_word(goal_word[p])')
replace('''    int n=0;while(a&&b&&(a&15)==(b&15)){++n;a>>=4;b>>=4;}return n;''', '''    int n=0;
    while(a&&b&&(a&15)==(b&15)) {
        int k=min(first_run_size(a),first_run_size(b));n+=k;a=suffix(a,k);b=suffix(b,k);
    }
    return n;''')
replace('for(TowerBits a=w;a;a>>=4)++actual[a&15];', 'for(TowerBits a=w;a;a>>=8)actual[a&15]+=first_run_size(a);')
replace('for(TowerBits w=initial[p];w;w>>=4)++initial_counts[w&15];', 'for(TowerBits w=initial[p];w;w>>=8)initial_counts[w&15]+=first_run_size(w);')
replace('for(TowerBits w=window.end[i];w;w>>=4)++required[w&15];', 'for(TowerBits w=window.end[i];w;w>>=8)required[w&15]+=first_run_size(w);')
replace('''                    for(int k=0;k<height(w);k++) {''', '''                    for(int k:RunCuts{w}) {''')
# 個体IDは元の便との対応にだけ使い、色盤面を塊へ復元する。
replace('''        for(int i=0;i<h[p];i++)w|=board_info.initial[a[p][i]]<<(4*i);''', '''        for(int i=0;i<h[p];i++)w=append_tower(w,board_info.initial[a[p][i]]);''')
replace('''        for(int i=h[p]-1;i>=m.k;i--)a[q][h[q]++]=a[p][i];''', '''        if(m.k&&board_info.initial[a[p][m.k-1]]==board_info.initial[a[p][m.k]])
            throw logic_error("atomic identity split");
        for(int i=h[p]-1;i>=m.k;i--)a[q][h[q]++]=a[p][i];''')
replace('''                Move adjusted=m;adjusted.k=keep;''', '''                if(keep&&board_info.initial[cur.a[p][keep-1]]==board_info.initial[cur.a[p][keep]]) {
                    LOCAL_ONLY(++atomic_stats.extract_split_prunes);return false;
                }
                Move adjusted=m;adjusted.k=keep;''')
# 従来の同色隙間共有を塊の列挙へ置き換える。移動の可否は別途境界条件で決める。
section('    void close_color_gaps(', '    int link(', r'''    void close_color_gaps(int p,TowerBits word,int h,int color_code,bool top_only) {
        ++alias_closures;
        if(h==0||h>=8)return;
        Label* labels=small_labels.data()+8*p;
        int first=0;
        for(TowerBits a=word;a;a>>=8) {
            int last=first+first_run_size(a);
            if(int(a&15)==color_code&&(!top_only||last==h)) {
                Label best=labels[first];
                for(int g=first+1;g<=last;g++)if(better(labels[g].cost,labels[g].bonus,best))best=labels[g];
                for(int g=top_only?last:first;g<=last;g++)if(better(best.cost,best.bonus,labels[g])) {
                    alias_cost_relaxations+=best.cost<labels[g].cost;
                    if(top_only)++alias_top_relaxations;else ++alias_gap_relaxations;
                    labels[g]=best;
                }
            }
            first=last;
        }
    }
    static TowerBits insert_token(TowerBits w,int gap,int color_code) {
        return insert_word(w,gap,TowerBits(color_code));
    }
''')
# 再挿入の自由移動と固定便の双方で、接合した同色の分離を止める。
replace('''                if(at.cost>=ceiling())continue;
                int next_cost''', '''                if(at.cost>=ceiling())continue;
                if(top_color_code(b[p])==color_code){LOCAL_ONLY(++atomic_stats.router_join_stops);continue;}
                int next_cost''')
replace('''            board_info.apply(b,m);h[p]=height(b[p]);h[q]=height(b[q]);''', '''            const TowerBits before_p=b[p];
            board_info.apply(b,m);h[p]=height(b[p]);h[q]=height(b[q]);''',2)
replace('''                const bool held=location==p&&added_keep;''', '''                const bool held=location==p&&added_keep;
                if(location==p&&!run_boundary(insert_token(before_p,gap,color_code),m.k+added_keep)) {
                    LOCAL_ONLY(++atomic_stats.insert_split_prunes);return;
                }''')
replace('''                if(at.cost!=queued_cost||at.cost>=ceiling())continue;
                int nc=at.cost+1,ri=rev[i];''', '''                if(at.cost!=queued_cost||at.cost>=ceiling())continue;
                if(top_color_code(b[p])==int(words[i]&15)){LOCAL_ONLY(++atomic_stats.router_join_stops);continue;}
                int nc=at.cost+1,ri=rev[i];''')
replace('''                const int add=held?n:0;''', '''                const int add=held?n:0;
                if(location==p&&!run_boundary(insert_word(before_p,gap,words[i]),m.k+add)) {
                    LOCAL_ONLY(++atomic_stats.insert_split_prunes);return;
                }''')
# 出発点の内部も、途中から除去する対象も塊単位で選ぶ。
replace('''                if(board_info.normalize(prefix(boundary[cell],k),cell)==prefix(boundary[cell],k))ks[count++]=k;''', '''                if(run_boundary(boundary[cell],k)&&board_info.normalize(prefix(boundary[cell],k),cell)==prefix(boundary[cell],k))ks[count++]=k;''')
replace('''                int n=int(board.h[p])-k;if(k<0||n<2)continue;''', '''                int n=int(board.h[p])-k;if(k<0||n<2||!run_boundary(word,k))continue;''')
# 独立再生も同色分割を検出し、出力操作の塊数を記録する。
replace('''        assert(0 <= k && k < h && 1 <= l && l <= k + 1);''', '''        assert(0 <= k && k < h && 1 <= l && l <= k + 1);
        assert(k==0||board_info.initial[tower[p][k-1]]!=board_info.initial[tower[p][k]]);''')
replace('''    trace.count_by("T", answer.size());''', '''    trace.count_by("atomic_output_same_color_splits",0);
    trace.count_by("T", answer.size());''')
# 制約による不可能な候補は明示的な棄却とし、既存の例外回復は廃止する。
replace('''        LOCAL_NOTE(trace.count_by("baseline_recovery", 1);)cerr<<"baseline diagnostic: "<<e.what()<<'\\n';
        ConstructionState w(board_info.initial);finish_runs(w);best=move(w.moves);baseline=best.size();''', '''        cerr<<"baseline diagnostic: "<<e.what()<<'\\n';return 1;''')
replace('''         catch(const exception& e){LOCAL_NOTE(trace.count_by("construction_errors", 1);)cerr<<"construction diagnostic: "<<e.what()<<'\\n';}''', '''         catch(const InfeasibleConstruction&){LOCAL_ONLY(++atomic_stats.blocked_constructions);}
         catch(const exception& e){cerr<<"construction diagnostic: "<<e.what()<<'\\n';return 1;}''')
replace('''    catch(const exception& e){LOCAL_NOTE(trace.count_by("lns_errors", 1);)cerr<<"LNS diagnostic: "<<e.what()<<'\\n';}''', '''    catch(const exception& e){cerr<<"LNS diagnostic: "<<e.what()<<'\\n';return 1;}''')
replace('''        LOCAL_NOTE(trace.count_by("final_recovery", 1);)cerr<<"final diagnostic: "<<e.what()<<'\\n';
        ConstructionState w(board_info.initial);finish_runs(w);best=move(w.moves);''', '''        cerr<<"final diagnostic: "<<e.what()<<'\\n';return 1;''')
replace('''        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();''', '''        atomic_stats.summary();
        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();''')

# 途中の塊を任意の隙間へ戻せるよう、束再挿入の開始位置を明示する。
replace('''                      TowerBits packet,int cap,vector<Move>& result) {''', '''                      TowerBits packet,int cap,vector<Move>& result,int initial_gap=-1) {''')
replace('''        labels[source*8+h[source]]={0,0,-1};''', '''        if(initial_gap<0)initial_gap=h[source];
        if(initial_gap>h[source])return false;
        labels[source*8+initial_gap]={0,0,-1};''')
replace('''                    const int ri=rev[ni];
                    for(int d=0;d<4;d++)''', '''                    // 帰巣を止めた背景と下端が同色なら、塊だけの即時再出発も分割になる。
                    if(int(words[ni]&15)==board_info.nest_code[tag_cell]) {
                        LOCAL_ONLY(++atomic_stats.insert_split_prunes);return;
                    }
                    const int ri=rev[ni];
                    for(int d=0;d<4;d++)''')
section('    // 途中の塔の上側を除去し、個体ごとに再挿入する。',
        '    // 同じ途中時点の2塔の上側を再構築する。', r'''    bool insert_run(const State& initial,const vector<Move>& base,int source,TowerBits piece,
                    int gap,int cap,vector<Move>& result) {
        assert(mono_color_code(piece)>0);
        LOCAL_ONLY(++atomic_stats.run_reinsertions;atomic_stats.run_reinserted_slimes+=height(piece));
        if(height(piece)==1)return insert_at(initial,base,source,int(piece&15),gap,cap,result);
        return insert_packet(initial,base,source,piece,cap,result,gap);
    }

    // 途中の上側を同色の塊へ分け、塊ごとに経路を作り直す。塊内部の個体順は探索しない。
    bool flexible_neighbor(const vector<Move>& current,const PacketCut& cut,int slack,
                          vector<Move>& result) {
        IdentityState cur=checkpoint_at(current,cut.time);
        int source=current[cut.time].p;
        TowerBits before=cur.word(source),packet=suffix(before,cut.keep);
        const int n=runs(packet);
        if(n<1||!run_boundary(before,cut.keep)||
           board_info.normalize(prefix(before,cut.keep),source)!=prefix(before,cut.keep))return false;
        array<TowerBits,8> pieces{};
        for(TowerBits a=packet; a; a>>=8)pieces[runs(packet)-runs(a)]=a&255;
        for(int i=cut.keep;i<cur.h[source];i++)cur.alive[cur.a[source][i]]=0;
        cur.h[source]=uint8_t(cut.keep);
        const State initial=cur.words();
        vector<Move> skeleton;
        if(!extract_from(current,cur,cut.time,skeleton))return false;
        const int allowance=int(current.size())-cut.time+slack;
        int best_length_value=allowance+1;
        vector<Move> best_tail;
        int mode=rng(4),rounds=n==1?1:(n<=4?2:1);
        for(int round=0;round<rounds;round++) {
            time_keeper.check();array<int,8> order{};iota(order.begin(),order.begin()+n,0);
            int style=(mode+round)%4;
            if(style==0)reverse(order.begin(),order.begin()+n);
            else if(style==2)stable_sort(order.begin(),order.begin()+n,[&](int a,int b){
                return board_info.floor_dist[source][board_info.nest_pos_by_code[pieces[a]&15]]>
                       board_info.floor_dist[source][board_info.nest_pos_by_code[pieces[b]&15]];
            });
            else if(style==3)for(int j=n-1;j>0;j--)swap(order[j],order[rng(j+1)]);
            if(board_info.nest_code[source]) {
                auto at=find(order.begin(),order.begin()+n,n-1);rotate(order.begin(),at,at+1);
            }
            State restored=initial;unsigned inserted=0;vector<Move> tail=skeleton;bool ok=true;
            for(int j=0;j<n;j++) {
                int rank=order[j],gap=cut.keep;
                for(int k=0;k<rank;k++)if(inserted>>k&1)gap+=height(pieces[k]);
                TowerBits next_initial=insert_word(restored[source],gap,pieces[rank]);
                if(board_info.normalize(next_initial,source)!=next_initial){ok=false;break;}
                vector<Move> next;int cap=min(allowance,best_length_value-1)-int(tail.size());
                if(!insert_run(restored,tail,source,pieces[rank],gap,cap,next)){ok=false;break;}
                ++insertions;tail=move(next);restored[source]=next_initial;inserted|=1u<<rank;
            }
            if(ok&&restored[source]==before&&int(tail.size())<best_length_value) {
                best_length_value=int(tail.size());best_tail=move(tail);
            }
        }
        if(best_length_value>allowance)return false;
        result.assign(current.begin(),current.begin()+cut.time);
        result.insert(result.end(),best_tail.begin(),best_tail.end());return true;
    }

''')
replace('''    // 同じ途中時点の2塔の上側を再構築する。各個体の出発点と元の段を保持し、''', '''    // 同じ途中時点の2塔の上側を再構築する。各塊の出発点と元の段を保持し、''')
replace('''        struct Member {int source,rank,shade,id;};''', '''        struct Member {int source,rank,shade;TowerBits word;};''')
replace('''            for(int k=keep;k<cur.h[cell];k++) {
                int id=cur.a[cell][k];
                members[n++]={cell,k,int(board_info.initial[id]),id};
                key=(key^uint64_t(id+1))*0x9e3779b97f4a7c15ULL;
                cur.alive[id]=0;
            }''', '''            int rank=keep;
            for(TowerBits rest=suffix(boundary[cell],keep);rest;rest>>=8) {
                members[n++]={cell,rank,int(rest&15),rest&255};rank+=first_run_size(rest);
            }
            for(int k=keep;k<cur.h[cell];k++) {
                int id=cur.a[cell][k];key=(key^uint64_t(id+1))*0x9e3779b97f4a7c15ULL;cur.alive[id]=0;
            }''')
replace('''for(int k=0;k<n;k++)if((inserted>>k&1)&&members[k].source==m.source&&members[k].rank<m.rank)++gap;
                TowerBits next_initial=insert_token(restored[m.source],gap,m.shade);''', '''for(int k=0;k<n;k++)if((inserted>>k&1)&&members[k].source==m.source&&members[k].rank<m.rank)gap+=height(members[k].word);
                TowerBits next_initial=insert_word(restored[m.source],gap,m.word);''')
replace('''if(!insert_at(restored,tail,m.source,m.shade,gap,cap,next))''', '''if(!insert_run(restored,tail,m.source,m.word,gap,cap,next))''')
replace('''// 塔全体、跳ぶ束、上側2匹を候補にする。再挿入時には個体へ分解できる。''', '''// 塔全体、跳ぶ束、上側2匹を候補にし、同色の塊を途中で切る候補は除く。''')
# 分割後は残った塊が背景になる。現在盤面から配送を再計画し、古い背景で後半を発行しない。
section('    void emit(ConstructionState &w,TowerBits word,int p) {', 'public:\n    DeliveryDP', '')
replace('''    void finish(ConstructionState &w,int p) { TowerBits a=w.state[p]; get(a); emit(w,a,p); }
''', '')
replace('''            dp.finish(w,p);''', '''            dp.first_event(w,p);''')
replace('''            DeliveryDP dp(w.state,p);dp.finish(w,p);''', '''            DeliveryDP dp(w.state,p);dp.first_event(w,p);''')
section('    void delivery_event(int p) {', '    int select_delivery()', '''    void delivery_event(int p) {
        State before=w.state;
        DeliveryDP dp(w.state,p);
        // 分割・帰巣の直後に塊と足場が変わるため、次の配送は更新後の盤面で計画する。
        dp.first_event(w,p);
        sync(before,++family_counter);
    }
''')
replace('// 1束の全分割位置を評価する。上側を帰巣させてから下側を扱うため、未処理部分の容量を保てる。',
        '// 同色の塊の境界だけを評価し、最初の分割か帰巣までの操作を発行する。')
replace('''            TowerBits suffix=matched==8?0:suffix(w,matched);
            for(unsigned mask=color_mask(suffix);mask;mask&=mask-1)''', '''            TowerBits unmatched=matched==8?0:suffix(w,matched);
            for(unsigned mask=color_mask(unmatched);mask;mask&=mask-1)''')
replace('''inline int color_at(TowerBits a,int k) { return int(suffix(a,k)&15); }
''','')
replace('''class TemporalLNS {
''','''class TemporalLNS {
    friend struct AtomicProbe;
''')
assert s.endswith('}\n')
s=s[:-2]+'    return 0;\n}\n'
assert not re.search(r'>>\(4\*|tower_mask',s)
assert (ROOT/'notes/experiments/v213.md').exists()
SOURCE.write_text(s)
print('v213 source generated; solver executions=0')
