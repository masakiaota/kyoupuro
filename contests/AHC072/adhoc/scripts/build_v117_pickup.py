#!/usr/bin/env python3
from check_v113_integrated import block
from v089_data import ROOT, save, sha

RUN=ROOT/'results/nn_rank/v117/20261004_pickup_studio'
CONDITIONS={'pickup':.15}
STATS=r'''
struct PickupStats {
    int64_t calls=0,first_completed=0,completed=0,selected=0,raw_saved=0;
    double seconds=0;
    void summary() const {
        LOCAL_ONLY(
            trace.count_by("pickup_calls",calls);trace.count_by("pickup_first_completed",first_completed);
            trace.count_by("pickup_completed",completed);trace.count_by("pickup_selected",selected);
            trace.count_by("pickup_raw_saved",raw_saved);trace.add_time_ms("pickup",seconds*1000.0);
        );
    }
} pickup_stats;
'''

METHOD=r'''
    // 訪問済みかどうかを状態に持つ。同色の役割交換も同じ訪問状態の中だけで行う。
    bool insert_via(const State& initial,const vector<Move>& base,int token,int via,
                    int cap,vector<Move>& result) {
        if(cap<0||initial[token])return false;
        const int color_code=int(board_info.initial[token]),cell_count=board_info.cell_count;
        const int stride=8*cell_count,GOAL=2*stride;
        State b=initial;array<int,max_cells> h{};
        for(int p=0;p<cell_count;++p)h[p]=height(b[p]);
        auto& label=small_labels;label.assign(GOAL+1,Label());
        links.clear();links.reserve(32768);
        const int start_phase=via<0||token==via;
        label[start_phase*stride+token*8]={0,0,-1};
        auto& queue=small_queue;queue.prepare(2*cell_count);queue.reset(cap);
        auto ceiling=[&](){return min(cap,label[GOAL].cost);};
        auto close=[&](int phase,int p,bool top_only) {
            const int height=h[p];const TowerBits word=b[p];
            if(height==0||height>=8)return;
            Label* a=label.data()+phase*stride+8*p;
            if(top_only) {
                if(int((word>>(4*(height-1)))&15)!=color_code)return;
                int first=height-1;
                while(first>0&&int((word>>(4*(first-1)))&15)==color_code)--first;
                Label best=a[height];for(int g=first;g<height;++g)if(better(a[g].cost,a[g].bonus,best))best=a[g];
                if(better(best.cost,best.bonus,a[height]))a[height]=best;
                return;
            }
            for(int first=0;first<height;) {
                if(int((word>>(4*first))&15)!=color_code){++first;continue;}
                int last=first+1;while(last<height&&int((word>>(4*last))&15)==color_code)++last;
                Label best=a[first];for(int g=first+1;g<=last;++g)if(better(a[g].cost,a[g].bonus,best))best=a[g];
                for(int g=first;g<=last;++g)if(better(best.cost,best.bonus,a[g]))a[g]=best;
                first=last;
            }
        };
        auto seed=[&](int phase,int p) {
            if(p<0||h[p]>=8)return;
            const Label a=label[phase*stride+8*p+h[p]];
            if(a.cost<=ceiling())queue.push(phase*cell_count+p,a.cost);
        };
        auto neighborhood=[&](int phase,int p,bool changed) {
            seed(phase,p);if(!changed)return;
            for(int d=0;d<4;++d)for(int l=1;l<=board_info.ray_count[p][d];++l) {
                const int q=board_info.ray[p][d][l];if(h[q]<8&&l<=h[q]+1)seed(phase,q);
            }
        };
        close(start_phase,token,true);seed(start_phase,token);
        for(int t=0;t<=int(base.size());++t) {
            if((t&31)==0)time_keeper.check();
            int pops=0,queued,queued_cost;
            while(queue.pop(ceiling(),queued,queued_cost)) {
                const int phase=queued/cell_count,p=queued%cell_count;
                const Label at=label[phase*stride+8*p+h[p]];
                if(at.cost!=queued_cost||h[p]>=8||at.cost>=ceiling())continue;
                const int next_cost=at.cost+1;
                for(int d=0;d<4;++d)for(int l=1,far=min(h[p]+1,int(board_info.ray_count[p][d]));l<=far;++l) {
                    const int q=board_info.ray[p][d][l];if(h[q]>=8)continue;
                    const int next_phase=phase||q==via;
                    const bool gone=board_info.nest_code[q]==color_code;
                    if(gone&&!next_phase)continue;
                    const int ns=gone?GOAL:next_phase*stride+8*q+h[q];
                    if(better(next_cost,at.bonus,label[ns])) {
                        const Move m{int16_t(p),uint8_t(h[p]),uint8_t(d),uint8_t(l)};
                        label[ns]={next_cost,at.bonus,link(at.path,t,m,true)};
                        if(!gone)queue.push(next_phase*cell_count+q,next_cost);
                    }
                }
                if((++pops&1023)==0)time_keeper.check();
            }
            if(t==int(base.size()))break;
            queue.reset(ceiling());
            const Move m=base[t];const int p=m.p,q=board_info.ray[p][m.d][m.l];
            const int hp=h[p],hq=h[q];
            const int support_need=support_weight?shortcut_support_need(b,base,t,q,hq):infinite_cost;
            array<array<Label,8>,2> from_p,from_q;
            for(int phase=0;phase<2;++phase) {
                close(phase,p,false);close(phase,q,false);
                for(int g=0;g<8;++g) {
                    from_p[phase][g]=label[phase*stride+8*p+g];
                    from_q[phase][g]=label[phase*stride+8*q+g];
                    label[phase*stride+8*p+g]=Label();label[phase*stride+8*q+g]=Label();
                }
            }
            board_info.apply(b,m);h[p]=height(b[p]);h[q]=height(b[q]);
            auto transition=[&](int phase,int location,int gap,int added_keep,const Label& at) {
                if(at.cost>ceiling())return;
                const bool held=location==p&&added_keep;
                if(!held&&hq+hp-m.k+1>8)return;
                const int tag_cell=held?p:q,tag_index=location==p&&!held?hq+hp-gap:gap;
                const int next_phase=phase||tag_cell==via;
                const bool gone=board_info.nest_code[tag_cell]==color_code&&tag_index>=h[tag_cell];
                if(gone&&!next_phase)return;
                int bonus=at.bonus;if(location==p)bonus+=held?support_weight:ride_weight;
                if(held&&support_need<=1)bonus+=3*support_weight;
                Move adjusted=m;adjusted.k+=added_keep;
                if(gone||tag_index<=h[tag_cell]) {
                    if(!gone&&h[tag_cell]>=8)return;
                    const int ns=gone?GOAL:next_phase*stride+8*tag_cell+tag_index;
                    if(better(at.cost,bonus,label[ns])) {
                        int ptr=at.path;if(added_keep)ptr=link(ptr,t,adjusted,false);
                        label[ns]={at.cost,bonus,ptr};
                    }
                } else if(at.cost<ceiling()) {
                    const int keep=tag_index,new_cost=at.cost+1,nb=bonus+3*keep;
                    int ptr=at.path;if(added_keep)ptr=link(ptr,t,adjusted,false);
                    for(int d=0;d<4;++d)for(int l=1,far=min(keep+1,int(board_info.ray_count[tag_cell][d]));l<=far;++l) {
                        const int dest=board_info.ray[tag_cell][d][l];if(h[dest]>=8)continue;
                        const int final_phase=next_phase||dest==via;
                        const bool home=board_info.nest_code[dest]==color_code;
                        if(home&&!final_phase)continue;
                        const int ns=home?GOAL:final_phase*stride+8*dest+h[dest];
                        if(!better(new_cost,nb,label[ns]))continue;
                        const Move escape{int16_t(tag_cell),uint8_t(keep),uint8_t(d),uint8_t(l)};
                        label[ns]={new_cost,nb,link(ptr,t+1,escape,true)};
                        if(!home)queue.push(final_phase*cell_count+dest,new_cost);
                    }
                }
            };
            for(int phase=0;phase<2;++phase) {
                if(hp<8)for(int g=0;g<=hp;++g) {
                    if(g>=m.k)transition(phase,p,g,0,from_p[phase][g]);
                    if(g<=m.k)transition(phase,p,g,1,from_p[phase][g]);
                }
                if(hq<8)for(int g=0;g<=hq;++g)transition(phase,q,g,0,from_q[phase][g]);
            }
            for(int phase=0;phase<2;++phase) {
                close(phase,p,true);close(phase,q,true);
                bool reopen_p=h[p]!=hp,reopen_q=h[q]!=hq;
                if(hp<8&&!reopen_p)reopen_p=better(from_p[phase][hp].cost,from_p[phase][hp].bonus,label[phase*stride+8*p+hp]);
                if(hq<8&&!reopen_q)reopen_q=better(from_q[phase][hq].cost,from_q[phase][hq].bonus,label[phase*stride+8*q+hq]);
                neighborhood(phase,p,reopen_p);neighborhood(phase,q,reopen_q);
            }
        }
        if(label[GOAL].cost>cap)return false;
        result=reconstruct(base,label[GOAL].path);
        if(result.size()!=base.size()+label[GOAL].cost)throw logic_error("pickup cost mismatch");
        return true;
    }

    double pickup_seconds=0;
    bool insert_pickup_order(const State& initial,const vector<Move>& base,
                             const vector<pair<double,int>>& order,int role,int allowance,vector<Move>& result) {
        ++pickup_stats.calls;const double began=time_keeper.exact_elapsed_sec();
        struct Measure {double began;double& seconds;
            ~Measure(){const double dt=time_keeper.exact_elapsed_sec()-began;seconds+=dt;pickup_stats.seconds+=dt;}
        } measure{began,pickup_seconds};
        const int first=order[role].second,partner=order[1-role].second;
        State restored=initial;vector<Move> tail;
        if(!insert_via(initial,base,first,partner,allowance-int(base.size()),tail))return false;
        ++pickup_stats.first_completed;restored[first]=board_info.initial[first];
        for(size_t j=1;j<order.size();++j) {
            const int id=j==1?partner:order[j].second;vector<Move> next;
            if(!insert(restored,tail,id,allowance-int(tail.size()),next))return false;
            tail=move(next);restored[id]=board_info.initial[id];
        }
        ++pickup_stats.completed;result=move(tail);return true;
    }
'''

def build():
    RUN.mkdir(parents=True,exist_ok=True)
    source=ROOT/'src/bin/v113_integrated_nn_lns.cpp';parent=source.read_text()
    assert sha(source)=='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
    text=parent.replace('class TemporalLNS {',STATS+'\nclass TemporalLNS {',1)
    text=text.replace('    // 第2順序では先頭2匹だけを交換する。',METHOD+'\n    // 第2順序では先頭2匹だけを交換する。',1)
    before=block(text,'    bool insert_two_orders(')
    after=before.replace('vector<Move>& result) {','vector<Move>& result,bool pickup=false) {',1)
    after=after.replace('        const bool ok=best_length_value<=allowance;',r'''
        if(pickup) {
            vector<Move> candidate;
            if(insert_pickup_order(initial,base,order,(iteration/4)&1,min(allowance,best_length_value-1),candidate)) {
                ++pickup_stats.selected;
                if(best_length_value<=allowance)pickup_stats.raw_saved+=best_length_value-int(candidate.size());
                best_length_value=int(candidate.size());result=move(candidate);
            }
        }
        const bool ok=best_length_value<=allowance;''',1)
    text=text.replace(before,after,1)
    text=text.replace('ok=insert_two_orders(initial,base,order,int(current.size())+4,rebuilt);',
                      'ok=insert_two_orders(initial,base,order,int(current.size())+4,rebuilt,\n'
                      '                        iteration%4==1 && pickup_seconds<0.15*\n'
                      '                        (active_seconds+time_keeper.exact_elapsed_sec()-slice_begin));',1)
    text=text.replace('        trace.count_by("floor_cells", board_info.cell_count);',
                      '        pickup_stats.summary();\n        trace.count_by("floor_cells", board_info.cell_count);',1)
    out=ROOT/'adhoc/bin/v117_pickup.cpp';body='// '+out.name+'\n'+text.split('\n',1)[1]
    if out.exists():assert out.read_text()==body
    out.write_text(body)
    save(RUN/'sources.json',dict(parent=sha(source),sources={'pickup':dict(path=str(out.relative_to(ROOT)),sha256=sha(out))}))

if __name__=='__main__':build()
