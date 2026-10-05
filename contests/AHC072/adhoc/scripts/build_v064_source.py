from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[2]
parent=root/'src/bin/v059_repair_priority.cpp'
original=parent.read_text();source=original;changes=[]
assert hashlib.sha256(parent.read_bytes()).hexdigest()=='a940c3b2c90e33d4425f531cacfd85286ff170a285fa09a97667c4bad121e002'
def change(old,new):
 global source
 assert source.count(old)==1,(old[:100],source.count(old))
 source=source.replace(old,new,1);changes.append(dict(old=old,new=new))
change('// v059_repair_priority.cpp','// v064_sparse_interval_lns.cpp')
fields='calls budget_skips no_candidate dead_ids no_window windows long_windows scope_filtered boundary_filtered extract_failed insert_failed completed equal shorter returned returned_saved long_completed long_shorter state_caps deadlines repaired scanned insert_calls links skipped_layers active_layers background_caps orders accepted accepted_equal best_updates best_saved max_span max_returned_span'.split()
header='''// v064: rebuild a bounded set of related events, even across a long span.
struct SparseIntervalStats {
    double seconds=0;
    uint64_t serial=0;
    int last_span=0,last_saved=0;
    int64_t '''+', '.join(k+'=0' for k in fields)+''';
    void summary() const {
        LOCAL_NOTE(
            auto count=[&](const char* name,int64_t value){trace.count_by(string("sparse_interval_")+name,value);};
'''+''.join(f'            count("{k}",{k});\n' for k in fields)+'''            trace.add_time_ms("sparse_interval",seconds*1000.0);
        )
    }
} sparse_interval_stats;
class TemporalLNS;
bool sparse_interval_neighbor(TemporalLNS&,const vector<Move>&,vector<Move>&,double);
// End v064 declarations.

'''
change('struct Removal {',header+'struct Removal {')
change('    int regularSelections=0;','    int regularSelections=0;\n    friend struct SparseIntervalRebuilder;\n    friend struct SparseIntervalAudit;')
change('            bool reorder=iteration%113==0;','''            const double sparse_allowance=0.03*max(PROGRAM_TIME_LIMIT_SEC*0.02,clk.elapsed()-lnsStart);
            const bool sparse_due=iteration%64==0;
            const bool sparse=sparse_due&&sparse_interval_stats.seconds<sparse_allowance;
            LOCAL_NOTE(sparse_interval_stats.budget_skips+=sparse_due&&!sparse;)
            bool reorder=!sparse&&iteration%113==0;''')
change('            bool paired=!reorder&&iteration%17==0&&','            bool paired=!sparse&&!reorder&&iteration%17==0&&')
change('            bool packet=!reorder&&!paired&&!packetCuts.empty()&&iteration%7==0&&','            bool packet=!sparse&&!reorder&&!paired&&!packetCuts.empty()&&iteration%7==0&&')
change('            bool dependency=!reorder&&!paired&&!packet&&++regularSelections%4==0;','            bool dependency=!sparse&&!reorder&&!paired&&!packet&&++regularSelections%4==0;')
change('            if(reorder) {\n                ++attempts;++reorderAttempts;','''            if(sparse) {
                ++attempts;
                const double begin=clk.elapsed();
                const double budget=min(PROGRAM_TIME_LIMIT_SEC*0.001,sparse_allowance-sparse_interval_stats.seconds);
                ok=sparse_interval_neighbor(*this,current,trial,min(end,begin+budget));
                sparse_interval_stats.seconds+=clk.elapsed()-begin;
            }else if(reorder) {
                ++attempts;++reorderAttempts;''')
change('                ++accepted;uphill+=delta>0;blockAccepted+=packet;flexibleAccepted+=flexible;pairedAccepted+=paired;','''                LOCAL_NOTE(if(sparse){
                    ++sparse_interval_stats.accepted;
                    sparse_interval_stats.accepted_equal+=delta==0;
                })
                ++accepted;uphill+=delta>0;blockAccepted+=packet;flexibleAccepted+=flexible;pairedAccepted+=paired;''')
change('                    int saved=int(best.size()-polished.size());','''                    int saved=int(best.size()-polished.size());
                    LOCAL_NOTE(if(sparse){++sparse_interval_stats.best_updates;sparse_interval_stats.best_saved+=saved;})''')
change('trace.count_by("lns_regular_saved", !packet && !reorder && !paired ? saved : 0);','trace.count_by("lns_regular_saved", !sparse && !packet && !reorder && !paired ? saved : 0);')
probe=(root/'adhoc/bin/v063_sparse_lns_probe.cpp').read_text()
body=probe[probe.index('    bool setScope('):probe.index('    static vector<vector<int>> candidates(')]
# Keep the same finite reinsertion transitions, but share scratch arrays with
# the caller and keep local flight identities separate from its current plan.
body=body.replace('lns.flights','window_flights').replace('throw StateCap{}','throw SparseIntervalStateCap{}')
body=body.replace('++stats.', '++stats.')
# All diagnostic work is absent in the production build.
body=body.replace('++stats.insert_calls;', 'LOCAL_NOTE(++stats.insert_calls;)')
body=body.replace('++stats.links;', 'LOCAL_NOTE(++stats.links;)')
body=body.replace('++stats.background_caps;', 'LOCAL_NOTE(++stats.background_caps;)')
body=body.replace('stats.scanned+=t<int(base.size());', 'LOCAL_NOTE(stats.scanned+=t<int(base.size());)')
body=body.replace('++stats.skipped_layers;', 'LOCAL_NOTE(++stats.skipped_layers;)')
body=body.replace('++stats.active_layers;', 'LOCAL_NOTE(++stats.active_layers;)')
body=body.replace('++stats.scanned;', 'LOCAL_NOTE(++stats.scanned;)')
body=body.replace('++stats.repaired;', 'LOCAL_NOTE(++stats.repaired;)')
body=body.replace('if(sparse&&!allowed[p]&&!allowed[q]) {','if(!allowed[p]&&!allowed[q]) {')
# The audit can compare the dense DP without leaving a runtime branch in submissions.
body=body.replace('                if(!allowed[p]&&!allowed[q]) {','''                if(!allowed[p]&&!allowed[q]
#ifdef AHC072_SPARSE_AUDIT
                   && skip_independent
#endif
                ) {''')
body=body.replace('        for(const auto& use:', '        for(const auto& use:')
body=body.replace('        for(Move m:segment) {\n            const int q=', '        for(Move m:segment) {\n            clk.check();\n            const int q=')
body=body.replace('        for(auto m:moves) {\n            TemporalLNS::Flight f;', '        for(auto m:moves) {\n            clk.check();\n            TemporalLNS::Flight f;')
core='''// v064 implementation. Each accepted interval restores the full endpoint.
struct SparseIntervalStateCap {};
struct SparseIntervalRebuilder {
    TemporalLNS& lns;
    SparseIntervalStats& stats=sparse_interval_stats;
    vector<TemporalLNS::Flight> window_flights;
    array<bool,MV> allowed{};
    int scope_cells=0,scope_operations=0;
#ifdef AHC072_SPARSE_AUDIT
    bool skip_independent=true;
#endif
    explicit SparseIntervalRebuilder(TemporalLNS& engine):lns(engine){allowed.fill(true);}
'''+body+'''
    // Select only from the current solution. Neither reference answers nor
    // case identifiers participate in the neighborhood definition.
    bool select(const vector<Move>& plan,uint64_t serial,vector<int>& ids,int& begin,int& end,
                IdentityBoard& entry,IdentityBoard& exit) {
        vector<int> eligible;
        for(int i=0;i<int(lns.candidates.size())&&eligible.size()<64;i++)
            if(!lns.candidates[i].ids.empty()&&lns.candidates[i].ids.size()<=4)eligible.push_back(i);
        if(eligible.empty()){LOCAL_NOTE(++stats.no_candidate;)return false;}
        ids=lns.candidates[eligible[serial%eligible.size()]].ids;
        array<bool,MV> selected{};for(int id:ids)selected[id]=true;
        vector<int> anchors;
        for(int t=0;t<int(lns.flights.size());t++) {
            if((t&63)==0)clk.check();
            bool hit=false;for(int j=0;j<lns.flights[t].n;j++)hit|=selected[lns.flights[t].id[j]];
            if(hit)anchors.push_back(t);
        }
        if(anchors.empty()){LOCAL_NOTE(++stats.no_window;)return false;}
        begin=anchors[(serial*17)%anchors.size()];
        entry=lns.checkpoints[begin/TemporalLNS::CHECKPOINT];
        for(int t=begin/TemporalLNS::CHECKPOINT*TemporalLNS::CHECKPOINT;t<begin;t++)entry.apply(plan[t]);
        for(int id:ids)if(!entry.alive[id]){LOCAL_NOTE(++stats.dead_ids;)return false;}
        IdentityBoard board=entry;
        auto contains=[&](int p){for(int j=0;j<board.h[p];j++)if(selected[board.a[p][j]])return true;return false;};
        array<bool,MV> region{};
        for(int p=0;p<geo.V;p++)if(contains(p))region[p]=true;
        int cells=count(region.begin(),region.begin()+geo.V,true),related=0;
        end=begin;exit=entry;
        for(int t=begin;t<int(plan.size());t++) {
            clk.check();Move m=plan[t];const int q=geo.ray[m.p][m.d][m.l];
            auto expanded=region;int count_cells=cells;
            if(contains(m.p)||contains(q))for(int p:{int(m.p),q})if(!expanded[p]){expanded[p]=true;++count_cells;}
            if(count_cells>12)break;
            int count_related=related;
            if(count_cells!=cells) {
                // Newly admitted cells can touch earlier background events.
                count_related=0;
                for(int u=begin;u<t;u++) {
                    if((u&63)==0)clk.check();
                    Move z=plan[u];count_related+=expanded[z.p]||expanded[geo.ray[z.p][z.d][z.l]];
                }
            }
            const bool hit=expanded[m.p]||expanded[q];count_related+=hit;
            if(count_related>32)break;
            region=expanded;cells=count_cells;related=count_related;board.apply(m);
            if(hit){end=t+1;exit=board;}
            bool alive=false;for(int id:ids)alive|=board.alive[id];if(!alive)break;
        }
        if(end-begin<2){LOCAL_NOTE(++stats.no_window;)return false;}
        return true;
    }

    // A completed result survives a later deadline. No partially reinserted
    // state is ever exposed to the caller.
    bool rebuild(const IdentityBoard& entry,const IdentityBoard& exit,const vector<Move>& original,
                 const vector<int>& ids,vector<Move>& result) {
        bool found=false;
        try {
            clk.check();
            if(!setScope(entry,original,ids)){LOCAL_NOTE(++stats.scope_filtered;)return false;}
            LOCAL_NOTE(++stats.windows;stats.long_windows+=original.size()>64;
                       stats.max_span=max<int64_t>(stats.max_span,original.size());)
            flights(entry,original);
            set<int> removed(ids.begin(),ids.end());IdentityBoard cur=without(entry,removed);
            const auto background=words(cur),goal=words(without(exit,removed));
            if(!normalized(background)||!normalized(goal)){LOCAL_NOTE(++stats.boundary_filtered;)return false;}
            vector<Move> base;
            if(!extract(original,cur,goal,base)){LOCAL_NOTE(++stats.extract_failed;)return false;}
            auto order=ids;
            do {
                clk.check();LOCAL_NOTE(++stats.orders;)
                vector<Move> tail=base;set<int> pending=removed;bool complete=true;
                try {
                    for(int id:order) {
                        clk.check();
                        const auto initial=words(without(entry,pending));
                        auto [source,gap]=position(entry,id,pending);
                        auto [target_cell,target_gap]=position(exit,id,pending);
                        pending.erase(id);
                        const auto restored=words(without(entry,pending)),target=words(without(exit,pending));
                        if(!normalized(restored)||!normalized(target)){
                            LOCAL_NOTE(++stats.boundary_filtered;)complete=false;break;
                        }
                        vector<Move> next;bool inserted;
                        {Board b=initial;inserted=insertAt(b,tail,source,int(geo.initial[id]),gap,
                            int(original.size())-int(tail.size()),target_cell,target_gap,next);}
                        if(!inserted){LOCAL_NOTE(++stats.insert_failed;)complete=false;break;}
                        if(!same(replay(restored,next),target))throw logic_error("sparse reinsertion endpoint mismatch");
                        tail=move(next);
                    }
                }catch(const SparseIntervalStateCap&){LOCAL_NOTE(++stats.state_caps;)complete=false;}
                if(!complete)continue;
                if(!same(replay(words(entry),tail),words(exit)))throw logic_error("sparse interval endpoint mismatch");
                LOCAL_NOTE(++stats.completed;stats.long_completed+=original.size()>64;
                    stats.equal+=tail.size()==original.size();stats.shorter+=tail.size()<original.size();
                    stats.long_shorter+=original.size()>64&&tail.size()<original.size();)
                if(!TemporalLNS::sameMoves(tail,original)&&(!found||tail.size()<result.size())){
                    result=move(tail);found=true;
                }
            }while(next_permutation(order.begin(),order.end()));
        }catch(const Deadline&){LOCAL_NOTE(++stats.deadlines;)}
        return found;
    }
    bool run(const vector<Move>& plan,vector<Move>& result,uint64_t serial) {
        vector<int> ids;int begin=0,end=0;IdentityBoard entry,exit;
        if(!select(plan,serial,ids,begin,end,entry,exit))return false;
        vector<Move> segment(plan.begin()+begin,plan.begin()+end),changed;
        if(!rebuild(entry,exit,segment,ids,changed))return false;
        result.clear();result.reserve(plan.size()-segment.size()+changed.size());
        result.insert(result.end(),plan.begin(),plan.begin()+begin);
        result.insert(result.end(),changed.begin(),changed.end());
        result.insert(result.end(),plan.begin()+end,plan.end());
        stats.last_span=end-begin;stats.last_saved=int(segment.size())-int(changed.size());
        LOCAL_NOTE(++stats.returned;stats.returned_saved+=stats.last_saved;
                   stats.max_returned_span=max<int64_t>(stats.max_returned_span,stats.last_span);)
        return true;
    }
};
bool sparse_interval_neighbor(TemporalLNS& lns,const vector<Move>& plan,vector<Move>& result,double deadline) {
    auto& stats=sparse_interval_stats;
    LOCAL_NOTE(++stats.calls;)
    stats.last_span=stats.last_saved=0;
    const uint64_t serial=stats.serial++;
    // Restore the parent's deadline on every exit, including exceptions.
    struct LimitGuard {double old=clk.limit;~LimitGuard(){clk.limit=old;}} guard;
    clk.limit=min(clk.limit,deadline);
    try {
        clk.check();SparseIntervalRebuilder rebuild(lns);
        return rebuild.run(plan,result,serial);
    }catch(const Deadline&){LOCAL_NOTE(++stats.deadlines;)return false;}
}
// End v064 implementation.

'''
change('#ifdef LOCAL\n// Replay using individual identities',core+'#ifdef LOCAL\n// Replay using individual identities')
change('        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();','        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();\n        sparse_interval_stats.summary();')
out=root/'src/bin/v064_sparse_interval_lns.cpp';out.write_text(source)
(root/'adhoc/v064_audit/registered_changes.json').write_text(json.dumps(changes,ensure_ascii=False,indent=2)+'\n')
restored=source
for c in reversed(changes):
 assert restored.count(c['new'])==1,c['new'][:100]
 restored=restored.replace(c['new'],c['old'],1)
assert restored==original
print('Created',out,'bytes',len(source.encode()),'registered edits',len(changes))
