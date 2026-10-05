#!/usr/bin/env python3
"""v204に事前登録した同色集荷だけを加える。solverは実行しない。"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT = "v204_initial_race_tuned"
CHILD = "v208_mono_collection"
AUDIT = ROOT / "adhoc/v208_audit"


def main():
    source = ROOT / f"src/bin/{CHILD}.cpp"
    assert not source.exists()
    assert (ROOT / "notes/experiments/v208.md").exists()
    AUDIT.mkdir(exist_ok=True)
    text = (ROOT / f"src/bin/{PARENT}.cpp").read_text()
    changes = []

    def replace(old, new):
        nonlocal text
        assert text.count(old) == 1, old[:120]
        at = text.index(old)
        changes.append(dict(old=old, new=new, offset=at))
        text = text[:at]+new+text[at+len(old):]

    replace(f"// {PARENT}.cpp", f"// {CHILD}.cpp")
    replace("class Constructor {", r'''// 同色の3塔以上を一度に評価する。根はids[0]で、総数は塔の容量以内。
struct MonoCollectionCandidate {
    double priority=-1;
    array<int,8> ids{};
    int count=0,pieces=0;
    bool operator<(const MonoCollectionCandidate& other)const{return priority<other.priority;}
};
#ifdef LOCAL
struct MonoCollectionStats {
    int64_t roots=0,proposed=0,stale=0,planned=0,feasible=0,positive=0;
    int64_t adopted=0,adopted_sources=0,adopted_pieces=0,moves=0,potential_saved=0;
    int64_t blocked=0,deadlines=0,completed=0;
    double generation_seconds=0,planning_seconds=0;
    unordered_map<int,int> completed_adoptions;
    void summary()const {
        auto count=[&](const char* key,int64_t value){trace.count_by(string("mono_collection_")+key,value);};
        count("roots",roots);count("proposed",proposed);count("stale",stale);
        count("planned",planned);count("feasible",feasible);count("positive",positive);
        count("adopted",adopted);count("adopted_sources",adopted_sources);count("adopted_pieces",adopted_pieces);
        count("moves",moves);count("potential_saved",potential_saved);
        count("blocked",blocked);count("deadlines",deadlines);count("completed",completed);
        trace.add_time_ms("mono_collection_generation",generation_seconds*1000.0);
        trace.add_time_ms("mono_collection_planning",planning_seconds*1000.0);
    }
} local_collection;
struct MonoCollectionTimer {
    double& seconds;
    chrono::steady_clock::time_point start=chrono::steady_clock::now();
    explicit MonoCollectionTimer(double& value):seconds(value){}
    ~MonoCollectionTimer(){seconds+=chrono::duration<double>(chrono::steady_clock::now()-start).count();}
};
#endif

class Constructor {
#ifdef AHC072_MONO_COLLECTION_PROBE
    friend struct MonoCollectionProbe;
#endif''')
    replace("    priority_queue<PairCandidate> pq;", "    priority_queue<PairCandidate> pq;\n    priority_queue<MonoCollectionCandidate> collection_queue;")
    replace("    void sync(const State &before,int family) {", r'''    bool collection_live(const MonoCollectionCandidate& candidate)const {
        for(int j=0;j<candidate.count;j++)if(!groups[candidate.ids[j]].active)return false;
        return true;
    }
    void add_collection(int root_id) {
        const Group& root=groups[root_id];
        const int color=mono_color_code(root.word);
        if(!root.active||color<=0||root.h>6)return;
        LOCAL_NOTE(MonoCollectionTimer timer(local_collection.generation_seconds);)
        LOCAL_ONLY(local_collection.roots++);
        MonoCollectionCandidate current,best;
        current.ids[0]=root_id;current.count=1;current.pieces=root.h;
        vector<int> nearest(groups.size(),infinite_cost);
        int cost=0,old=root.value;
        while(current.count<8) {
            if(time_keeper.exact_elapsed_sec()>search_end){LOCAL_ONLY(local_collection.deadlines++);return;}
            const int last=groups[current.ids[current.count-1]].p;
            int pick=-1,distance=infinite_cost;
            for(int id=0;id<int(groups.size());id++) {
                const Group& g=groups[id];
                if(!g.active||mono_color_code(g.word)!=color||current.pieces+g.h>8)continue;
                bool excluded=false;
                for(int j=0;j<current.count;j++) {
                    const Group& selected=groups[current.ids[j]];
                    if(id==current.ids[j]||(g.family&&g.family==selected.family)){excluded=true;break;}
                }
                if(excluded)continue;
                nearest[id]=min(nearest[id],int(board_info.floor_dist[last][g.p]));
                if(nearest[id]<distance){distance=nearest[id];pick=id;}
            }
            if(pick<0)break;
            current.ids[current.count++]=pick;current.pieces+=groups[pick].h;
            cost+=distance;old+=groups[pick].value;
            const int gain=old-cost-root.value;
            if(current.count>=3&&gain>0) {
                current.priority=gain*weight[min(cost,2*max_cells)];
                if(current.priority>best.priority)best=current;
            }
        }
        if(best.count>=3){collection_queue.push(best);LOCAL_ONLY(local_collection.proposed++);}
    }
    MergePlan plan_collection(const MonoCollectionCandidate& candidate,int& gain) {
        LOCAL_NOTE(MonoCollectionTimer timer(local_collection.planning_seconds);)
        LOCAL_ONLY(local_collection.planned++);
        MergePlan result;gain=0;
        const int root=groups[candidate.ids[0]].p;
        const int color=mono_color_code(groups[candidate.ids[0]].word);
        array<int,max_cells> background{},parent{},distance{},previous{};
        array<bool,max_cells> tree{};
        int old=0;
        for(int p=0;p<board_info.cell_count;p++)background[p]=height(w.state[p]);
        for(int j=0;j<candidate.count;j++) {
            const Group& g=groups[candidate.ids[j]];
            background[g.p]=0;old+=g.value;
        }
        parent.fill(-1);tree[root]=true;
        // 木へ最も近い対象塔を順に接続する。既に通った辺を再利用して集荷費を減らす。
        for(;;) {
            if(time_keeper.exact_elapsed_sec()>search_end){LOCAL_ONLY(local_collection.deadlines++);return result;}
            bool complete=true;
            for(int j=0;j<candidate.count;j++)if(!tree[groups[candidate.ids[j]].p])complete=false;
            if(complete)break;
            distance.fill(-1);previous.fill(-1);
            int queue[max_cells],head=0,tail=0;
            for(int p=0;p<board_info.cell_count;p++)if(tree[p]){distance[p]=0;queue[tail++]=p;}
            while(head<tail) {
                const int p=queue[head++];
                for(int d=0;d<4;d++) {
                    const int next=board_info.adj[p][d];
                    if(next<0||distance[next]>=0||board_info.nest_code[next]==color)continue;
                    // どの部分木の束も通せる容量を確保し、対象外の塔を下段に保持する。
                    if(background[next]+candidate.pieces>8)continue;
                    distance[next]=distance[p]+1;previous[next]=p;queue[tail++]=next;
                }
            }
            int pick=-1;
            for(int j=0;j<candidate.count;j++) {
                const int p=groups[candidate.ids[j]].p;
                if(!tree[p]&&distance[p]>=0&&(pick<0||distance[p]<distance[pick]))pick=p;
            }
            if(pick<0){LOCAL_ONLY(local_collection.blocked++);return result;}
            for(int p=pick;!tree[p];p=previous[p]){tree[p]=true;parent[p]=previous[p];}
        }
        int order[max_cells],head=0,tail=0;order[tail++]=root;
        while(head<tail) {
            const int p=order[head++];
            for(int d=0;d<4;d++) {
                const int child=board_info.adj[p][d];
                if(child>=0&&parent[child]==p)order[tail++]=child;
            }
        }
        State trial=w.state;
        for(int i=tail-1;i>0;i--) {
            if(time_keeper.exact_elapsed_sec()>search_end){LOCAL_ONLY(local_collection.deadlines++);return MergePlan();}
            const int p=order[i];
            if(height(trial[p])==background[p])continue;
            int direction=0;
            while(board_info.adj[p][direction]!=parent[p])++direction;
            Move m{int16_t(p),uint8_t(background[p]),uint8_t(direction),1};
            board_info.apply(trial,m);result.moves.push_back(m);
        }
        LOCAL_ONLY(local_collection.feasible++);
        gain=old-int(result.moves.size())-pot.value(trial[root],root);
        if(gain>0) {
            result.score=gain*weight[min(int(result.moves.size()),2*max_cells)];
            LOCAL_ONLY(local_collection.positive++);
        }
        return result;
    }
    void sync(const State &before,int family) {''')
    replace("                add_pair(a,b);\n            }\n        }\n    }\n    void consider", "                add_pair(a,b);\n            }\n            add_collection(a);\n        }\n    }\n    void consider")
    replace("public:\n    Constructor(EmptyDP", "public:\n    LOCAL_NOTE(int collection_adoptions=0;)\n    Constructor(EmptyDP")
    replace("        int initial=groups.size();\n        for(int a=0;", "        int initial=groups.size();\n        for(int a=0;a<initial;a++) {\n            if(time_keeper.exact_elapsed_sec()>search_end)break;\n            add_collection(a);\n        }\n        for(int a=0;")
    replace("            if(selected.score>0) {\n                State before=w.state;\n                for(auto m:selected.moves) w.move(m);\n                sync(before,0);", r'''            bool collection_selected=false;
            [[maybe_unused]] int collection_gain=0;
            MonoCollectionCandidate collection;
            while(!collection_queue.empty()&&time_keeper.exact_elapsed_sec()<=search_end) {
                collection=collection_queue.top();collection_queue.pop();
                if(!collection_live(collection)){LOCAL_ONLY(local_collection.stale++);continue;}
                int gain=0;
                auto plan=plan_collection(collection,gain);
                if(plan.score>0&&plan.score>selected.score) {
                    selected=move(plan);collection_selected=true;collection_gain=gain;
                } else if(plan.score>0) {
                    // 今回は2塔合流が勝った候補も、実計画の評価値で次の選択に残す。
                    collection.priority=plan.score;collection_queue.push(collection);
                }
                break;
            }
            if(selected.score>0) {
                State before=w.state;
                for(auto m:selected.moves) w.move(m);
                LOCAL_ONLY(if(collection_selected) {
                    ++collection_adoptions;++local_collection.adopted;
                    local_collection.adopted_sources+=collection.count;
                    local_collection.adopted_pieces+=collection.pieces;
                    local_collection.moves+=selected.moves.size();
                    local_collection.potential_saved+=collection_gain;
                });
                sync(before,0);'''.replace("            bool collection_selected=false;", "            [[maybe_unused]] bool collection_selected=false;"))
    replace('            trace.count_by("race_seed_"+to_string(id)+"_origin",pool.entries[id].origin);', '''            trace.count_by("race_seed_"+to_string(id)+"_origin",pool.entries[id].origin);
            trace.count_by("race_seed_"+to_string(id)+"_collection_plans",
                pool.entries[id].origin<0?0:local_collection.completed_adoptions.at(pool.entries[id].origin));''')
    replace("            if(!result.E){completed++;initial_solutions.offer(result.moves,attempts-1);if(result.moves.size()<best.size())best=move(result.moves);}", '''            if(!result.E){
                LOCAL_ONLY(
                    local_collection.completed_adoptions[attempts-1]=solver.collection_adoptions;
                    if(solver.collection_adoptions>0)++local_collection.completed;
                );
                completed++;initial_solutions.offer(result.moves,attempts-1);if(result.moves.size()<best.size())best=move(result.moves);
            }''')
    replace("        local_nn.summary();trace.summary();", "        local_nn.summary();local_collection.summary();trace.summary();")
    source.write_text(text)
    (AUDIT / f"{CHILD}_changes.json").write_text(json.dumps(changes,ensure_ascii=False,indent=2)+"\n")
    print(f"Created {CHILD}; registered {len(changes)} changes; no solver executed.")


if __name__ == "__main__":
    main()
