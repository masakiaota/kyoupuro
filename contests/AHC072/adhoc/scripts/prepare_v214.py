#!/usr/bin/env python3
"""v210から局所再構築の同色塊版を作る。solverは実行しない。"""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT / 'src/bin/v210_collection_color_quotient.cpp'
CHILD = ROOT / 'src/bin/v214_local_color_runs.cpp'


def main():
    assert not (ROOT / 'adhoc/v214_audit/frozen.json').exists()
    s = PARENT.read_text()

    def replace(old, new):
        nonlocal s
        assert s.count(old) == 1, old[:120]
        s = s.replace(old, new)

    replace('''// v210_collection_color_quotient.cpp
// v210: v209の同色再挿入・候補保持と、v208の3塔以上の同色集荷構築を併用する。
// 集荷の計画と採否判定はv208からそのまま継承する。
// LOCAL・非LOCALとも時間管理、初期解選抜、探索と終了処理はv209を継承する。''', '''// v214_local_color_runs.cpp
// v214: 途中の1塔・2塔の再構築だけ、選択範囲の同色連続部分を塊単位で再挿入する。
// 塊はこの再挿入中だけ保持し、同色の背景を足場に残して離れることを許す。
// 盤面の32 bit表現、初期構築、通常近傍、時間配分と終了処理はv210を継承する。''')
    replace('int64_t local_single_insert_calls = 0, local_packet_insert_calls = 0;', '''int64_t local_single_insert_calls = 0, local_packet_insert_calls = 0;
int64_t local_run_flexible_calls=0,local_run_paired_calls=0;
int64_t local_run_reinsert_calls=0,local_run_reinserted_slimes=0;
int64_t local_run_multi_calls=0,local_run_multi_completed=0;
int64_t local_run_completed=0,local_run_completed_slimes=0;''')
    replace('''    int link(int prev,int time,Move m,bool extra) {''', '''    // 元の段と匹数を保持する。背景へ同色で接触してもこの単位へ取り込まない。
    struct RunMember {int16_t source;uint8_t rank,shade,count;};
    static int append_runs(array<RunMember,8>& members,int n,TowerBits word,int source,int rank) {
        while(word) {
            const int shade=word&15;int count=0;
            do {++count;word>>=4;} while(word&&int(word&15)==shade);
            members[n++]={int16_t(source),uint8_t(rank),uint8_t(shade),uint8_t(count)};
            rank+=count;
        }
        return n;
    }
    static TowerBits run_word(const RunMember& m) {
        return prefix(0x11111111u*m.shade,m.count);
    }
    static TowerBits insert_run_word(TowerBits w,int gap,const RunMember& m) {
        // 上端が8段のときも、32 bit幅のシフトを避けて元の順序を復元する。
        const uint64_t upper=(uint64_t(w)>>(4*gap))<<(4*(gap+m.count));
        return TowerBits(prefix(w,gap)|(uint64_t(run_word(m))<<(4*gap))|upper);
    }
    int link(int prev,int time,Move m,bool extra) {''')
    replace('''                      TowerBits packet,int cap,vector<Move>& result) {''', '''                      TowerBits packet,int cap,vector<Move>& result,int initial_gap=-1) {''')
    replace('''        labels[source*8+h[source]]={0,0,-1};''', '''        // 途中再構築では元の段順を保つため、既に戻した塊の下にも挿入する。
        const int gap=initial_gap<0?h[source]:initial_gap;
        if(gap>h[source])return false;
        labels[source*8+gap]={0,0,-1};''')
    old_start = s.index('    // 途中の塔の上側を除去し、個体ごとに再挿入する。')
    old_end = s.index('    // 同じ途中時点の2塔の上側を再構築する。', old_start)
    s = s[:old_start] + '''    bool insert_run(const State& initial,const vector<Move>& base,const RunMember& m,
                    int gap,int cap,vector<Move>& result) {
        LOCAL_NOTE(
            ++local_run_reinsert_calls;local_run_reinserted_slimes+=m.count;
            local_run_multi_calls+=m.count>1;
        )
        const bool ok=m.count==1?insert_at(initial,base,m.source,m.shade,gap,cap,result):
            insert_packet(initial,base,m.source,run_word(m),cap,result,gap);
        LOCAL_NOTE(
            local_run_completed+=ok;local_run_completed_slimes+=ok?m.count:0;
            local_run_multi_completed+=ok&&m.count>1;
        )
        return ok;
    }

    // 選択した上側だけを同色の塊へまとめる。切出し位置と背景の分割は親と同じ自由度を持つ。
    bool flexible_neighbor(const vector<Move>& current,const PacketCut& cut,int slack,
                          vector<Move>& result) {
        LOCAL_NOTE(++local_run_flexible_calls;)
        IdentityState cur=checkpoint_at(current,cut.time);
        int source=current[cut.time].p;
        TowerBits before=cur.word(source),packet=before>>(4*cut.keep);
        const int slime_count=height(packet);
        if(slime_count<2||slime_count>8||board_info.normalize(prefix(before,cut.keep),source)!=prefix(before,cut.keep))return false;
        array<RunMember,8> members{};
        const int n=append_runs(members,0,packet,source,cut.keep);
        for(int i=cut.keep;i<cur.h[source];i++)cur.alive[cur.a[source][i]]=0;
        cur.h[source]=uint8_t(cut.keep);
        const State initial=cur.words();
        vector<Move> skeleton;
        if(!extract_from(current,cur,cut.time,skeleton))return false;
        const int allowance=int(current.size())-cut.time+slack;
        int best_length_value=allowance+1;
        vector<Move> best_tail;
        // 追加順序の匹数条件は親を維持する。1塊では同じ順序の再試行を省く。
        int mode=rng(4),rounds=n>1&&slime_count<=4?2:1;
        for(int round=0;round<rounds;round++) {
            time_keeper.check();
            array<int,8> order{};
            iota(order.begin(),order.begin()+n,0);
            int style=(mode+round)%4;
            if(style==0)reverse(order.begin(),order.begin()+n);
            else if(style==2) {
                stable_sort(order.begin(),order.begin()+n,[&](int a,int b){
                    return board_info.floor_dist[source][board_info.nest_pos_by_code[members[a].shade]]>
                           board_info.floor_dist[source][board_info.nest_pos_by_code[members[b].shade]];
                });
            }else if(style==3) {
                for(int j=n-1;j>0;j--)swap(order[j],order[rng(j+1)]);
            }
            // 巣では帰巣しない元の最上段の塊を先に戻す。
            if(board_info.nest_code[source]) {
                auto at=find(order.begin(),order.begin()+n,n-1);
                rotate(order.begin(),at,at+1);
            }
            State restored=initial;unsigned inserted=0;
            vector<Move> tail=skeleton;bool ok=true;
            for(int j=0;j<n;j++) {
                int rank=order[j],gap=cut.keep;
                for(int k=0;k<rank;k++)if(inserted>>k&1)gap+=members[k].count;
                TowerBits next_initial=insert_run_word(restored[source],gap,members[rank]);
                if(board_info.normalize(next_initial,source)!=next_initial){ok=false;break;}
                vector<Move> next;
                int cap=min(allowance,best_length_value-1)-int(tail.size());
                if(!insert_run(restored,tail,members[rank],gap,cap,next)){ok=false;break;}
                ++insertions;tail=move(next);restored[source]=next_initial;inserted|=1u<<rank;
            }
            if(ok&&restored[source]==before&&int(tail.size())<best_length_value) {
                best_length_value=int(tail.size());best_tail=move(tail);
            }
        }
        if(best_length_value>allowance)return false;
        result.assign(current.begin(),current.begin()+cut.time);
        result.insert(result.end(),best_tail.begin(),best_tail.end());
        return true;
    }

''' + s[old_end:]
    replace('''    // 同じ途中時点の2塔の上側を再構築する。各個体の出発点と元の段を保持し、''', '''    // 同じ途中時点の2塔の上側を再構築する。各塊の出発点と元の段を保持し、''')
    replace('''        LOCAL_NOTE(local_late_paired_anchor=false;)''', '''        LOCAL_NOTE(local_late_paired_anchor=false;++local_run_paired_calls;)''')
    replace('''        struct Member {int source,rank,shade,id;};
        array<Member,8> members{};int n=0;''', '''        array<RunMember,8> members{};int n=0,slime_count=0;''')
    replace('''        for(auto [cell,keep]:{pair<int,int>{p,keep_p},pair<int,int>{q,keep_q}}) {
            for(int k=keep;k<cur.h[cell];k++) {
                int id=cur.a[cell][k];
                members[n++]={cell,k,int(board_info.initial[id]),id};''', '''        for(auto [cell,keep]:{pair<int,int>{p,keep_p},pair<int,int>{q,keep_q}}) {
            n=append_runs(members,n,boundary[cell]>>(4*keep),cell,keep);
            slime_count+=cur.h[cell]-keep;
            for(int k=keep;k<cur.h[cell];k++) {
                int id=cur.a[cell][k];''')
    replace('''        const int mode=rng(5),rounds=n<=3?2:1;''', '''        const int mode=rng(5),rounds=slime_count<=3?2:1;''')
    replace('''                    // 巣では元の最上段を先に置き、他の個体をその下に挿入して帰巣を防ぐ。''', '''                    // 巣では元の上側の塊から戻し、その下へ置く塊の帰巣を防ぐ。''')
    replace('''                for(int k=0;k<n;k++)if((inserted>>k&1)&&members[k].source==m.source&&members[k].rank<m.rank)++gap;
                TowerBits next_initial=insert_token(restored[m.source],gap,m.shade);''', '''                for(int k=0;k<n;k++)if((inserted>>k&1)&&members[k].source==m.source&&members[k].rank<m.rank)gap+=members[k].count;
                TowerBits next_initial=insert_run_word(restored[m.source],gap,m);''')
    replace('''                if(!insert_at(restored,tail,m.source,m.shade,gap,cap,next)){ok=false;break;}''', '''                if(!insert_run(restored,tail,m,gap,cap,next)){ok=false;break;}''')
    replace('''        trace.count_by("packet_insert_calls", local_packet_insert_calls);''', '''        trace.count_by("packet_insert_calls", local_packet_insert_calls);
        trace.count_by("local_run_flexible_calls",local_run_flexible_calls);
        trace.count_by("local_run_paired_calls",local_run_paired_calls);
        trace.count_by("local_run_reinsert_calls",local_run_reinsert_calls);
        trace.count_by("local_run_reinserted_slimes",local_run_reinserted_slimes);
        trace.count_by("local_run_multi_calls",local_run_multi_calls);
        trace.count_by("local_run_multi_completed",local_run_multi_completed);
        trace.count_by("local_run_completed",local_run_completed);
        trace.count_by("local_run_completed_slimes",local_run_completed_slimes);''')
    # 旧個体単位の開始盤面復元は、両呼出箇所を置換したため不要になる。
    replace('''    static TowerBits insert_token(TowerBits w,int gap,int color_code) {
        TowerBits upper=gap==7?0:(w>>(4*gap))<<(4*(gap+1));
        return prefix(w,gap)|(TowerBits(color_code)<<(4*gap))|upper;
    }
''', '')
    CHILD.write_text(s)
    print(CHILD)


if __name__ == '__main__':
    main()
