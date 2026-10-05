#!/usr/bin/env python3
"""v113から、最初の個体の同費用経路を二本残す版を作る。"""
import difflib
from pathlib import Path
from check_v113_integrated import block
from v089_data import ROOT, sha

RUN = ROOT / 'results/nn_rank/v131/20261005_path_pairs_studio'
BASE = ROOT / 'src/bin/v113_integrated_nn_lns.cpp'
SOURCE = ROOT / 'src/bin/v131_path_pairs.cpp'
BASE_SHA = '037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'


def once(text, old, new):
    assert text.count(old) == 1, old[:160]
    return text.replace(old, new, 1)


def build():
    assert sha(BASE) == BASE_SHA
    old = BASE.read_text()
    text = '// ' + SOURCE.name + '\n' + old.split('\n', 1)[1]
    text = once(text, '    bool insert_at(const State& initial,const vector<Move>& base,int source,int color_code,\n                  int initial_gap,int cap,vector<Move>& result) {',
        '''    template<bool CollectAlternative=false>
    bool insert_at(const State& initial,const vector<Move>& base,int source,int color_code,
                  int initial_gap,int cap,vector<Move>& result,vector<Move>* alternative=nullptr) {
        if constexpr(CollectAlternative){assert(alternative);alternative->clear();}''')
    text = once(text, '        label[source*8+initial_gap]={0,0,-1};', '''        label[source*8+initial_gap]={0,0,-1};
        // 中間状態は親と同じ一本。帰巣イベントの異なる完成経路だけ二本記録する。
        // イベントを区別しても全経路の上位二本にはならないため、用途を先頭個体に限定する。
        struct GoalOption {Label value;uint64_t event=~uint64_t(0);};
        array<GoalOption,2> goals{};
        auto goal_event=[](int t,Move m,bool extra) {
            return (uint64_t(extra)<<63)|(uint64_t(t)<<24)|(uint64_t(m.p)<<12)|
                   (uint64_t(m.k)<<8)|(uint64_t(m.d)<<4)|uint64_t(m.l);
        };
        auto wants_goal=[&](int ns,int cost,int bonus,uint64_t event) {
            if constexpr(!CollectAlternative)return false;
            if(ns!=GOAL)return false;
            for(const auto& g:goals)if(g.event==event)return better(cost,bonus,g.value);
            return better(cost,bonus,goals[1].value);
        };
        auto remember_goal=[&](int ns,int cost,int bonus,int path,uint64_t event) {
            if constexpr(CollectAlternative) {
                if(ns!=GOAL)return;
                int at=goals[0].event==event?0:1;
                if(better(cost,bonus,goals[at].value))goals[at]={{cost,bonus,path},event};
                if(better(goals[1].value.cost,goals[1].value.bonus,goals[0].value))swap(goals[0],goals[1]);
            }
        };''')
    text = once(text, '''                    if(better(next_cost,at.bonus,label[ns])) {
                        Move m={int16_t(p),uint8_t(h[p]),uint8_t(d),uint8_t(l)};
                        int ptr=link(at.path,t,m,true);
                        label[ns]={next_cost,at.bonus,ptr};
                        if(ns!=GOAL)queue.push(q,next_cost);
                    }''', '''                    Move m={int16_t(p),uint8_t(h[p]),uint8_t(d),uint8_t(l)};
                    const bool primary=better(next_cost,at.bonus,label[ns]);
                    const uint64_t event=CollectAlternative&&ns==GOAL?goal_event(t,m,true):0;
                    if(primary||wants_goal(ns,next_cost,at.bonus,event)) {
                        int ptr=link(at.path,t,m,true);
                        remember_goal(ns,next_cost,at.bonus,ptr,event);
                        if(primary) {
                            label[ns]={next_cost,at.bonus,ptr};
                            if(ns!=GOAL)queue.push(q,next_cost);
                        }
                    }''')
    text = once(text, '''                    if(better(at.cost,bonus,label[ns])) {
                        int ptr=at.path;
                        if(added_keep)ptr=link(ptr,t,adjusted,false);
                        label[ns]={at.cost,bonus,ptr};
                    }''', '''                    const bool primary=better(at.cost,bonus,label[ns]);
                    const uint64_t event=CollectAlternative&&ns==GOAL?goal_event(t,adjusted,false):0;
                    if(primary||wants_goal(ns,at.cost,bonus,event)) {
                        int ptr=at.path;
                        if(added_keep)ptr=link(ptr,t,adjusted,false);
                        remember_goal(ns,at.cost,bonus,ptr,event);
                        if(primary)label[ns]={at.cost,bonus,ptr};
                    }''')
    text = once(text, '''                        if(!better(new_cost,nb,label[ns]))continue;
                        Move escape={int16_t(tag_cell),uint8_t(keep),uint8_t(d),uint8_t(l)};
                        int np=link(ptr,t+1,escape,true);
                        label[ns]={new_cost,nb,np};
                        if(ns!=GOAL)queue.push(dest,new_cost);''', '''                        Move escape={int16_t(tag_cell),uint8_t(keep),uint8_t(d),uint8_t(l)};
                        const bool primary=better(new_cost,nb,label[ns]);
                        const uint64_t event=CollectAlternative&&ns==GOAL?goal_event(t+1,escape,true):0;
                        if(!primary&&!wants_goal(ns,new_cost,nb,event))continue;
                        int np=link(ptr,t+1,escape,true);
                        remember_goal(ns,new_cost,nb,np,event);
                        if(primary) {
                            label[ns]={new_cost,nb,np};
                            if(ns!=GOAL)queue.push(dest,new_cost);
                        }''')
    text = once(text, '''        if(int(result.size())!=int(base.size())+label[GOAL].cost)
            throw logic_error("reinsertion cost mismatch");
        return true;''', '''        if(int(result.size())!=int(base.size())+label[GOAL].cost)
            throw logic_error("reinsertion cost mismatch");
        if constexpr(CollectAlternative) {
            assert(goals[0].value.path==label[GOAL].path);
            if(goals[1].value.cost==label[GOAL].cost) {
                *alternative=reconstruct(base,goals[1].value.path);
                if(alternative->size()!=result.size())throw logic_error("alternate insertion cost mismatch");
                if(same_moves(result,*alternative))alternative->clear();
            }
        }
        return true;''')
    previous = block(text, '    bool insert_two_orders(')
    replacement = (ROOT / 'adhoc/scripts/v131_two_paths.cpp.txt').read_text().rstrip()
    text = once(text, previous, replacement)
    text = once(text, '            vector<Move> trial;\n', '            vector<Move> trial;bool trial_smoothed=false;\n')
    text = once(text, 'ok=insert_two_orders(initial,base,order,int(current.size())+4,rebuilt);',
                'ok=insert_two_orders(initial,base,order,int(current.size())+4,rebuilt,trial_smoothed);')
    text = once(text, 'try{polished=smooth_routes(trial);}catch(const exception&)',
                'try{polished=trial_smoothed?move(trial):smooth_routes(trial);}catch(const exception&)')
    # 移動したvectorの長さを短縮計数へ使わない。
    text = once(text, '            shortcuts+=int(trial.size())-int(polished.size());',
                '            if(!trial_smoothed)shortcuts+=int(trial.size())-int(polished.size());')
    if SOURCE.exists():
        assert SOURCE.read_text() == text
    else:
        SOURCE.write_text(text)
    RUN.mkdir(parents=True, exist_ok=True)
    (RUN / 'source.patch').write_text(''.join(difflib.unified_diff(old.splitlines(True), text.splitlines(True), fromfile=BASE.name, tofile=SOURCE.name)))
    return SOURCE


if __name__ == '__main__':
    print(build())
