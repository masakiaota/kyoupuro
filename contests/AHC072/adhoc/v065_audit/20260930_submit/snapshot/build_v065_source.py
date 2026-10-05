#!/usr/bin/env python3
"""v059への登録差分からv065を作る。solverは実行しない。"""
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

change('// v059_repair_priority.cpp','// v065_bounded_interval_lns.cpp')
fields='calls budget_skips no_window windows windows_16 windows_32 windows_64 candidates sets_tried boundary_filtered extract_failed insert_failed completed equal shorter returned returned_saved returned_outside outside_saved state_caps deadlines repaired scanned insert_calls links orders accepted accepted_equal accepted_outside best_updates best_saved max_span max_returned_span'.split()
header='''// v065: short intervals permit reinsertion beyond the original route.
struct BoundedIntervalStats {
    double seconds=0,preparation_seconds=0;
    uint64_t serial=0;
    int last_span=0,last_saved=0,last_begin=0,last_candidates=0;
    bool last_outside=false;
    int64_t '''+', '.join(k+'=0' for k in fields)+''';
    void summary() const {
        LOCAL_NOTE(
            auto count=[&](const char* name,int64_t value){trace.count_by(string("bounded_interval_")+name,value);};
'''+''.join(f'            count("{k}",{k});\n' for k in fields)+'''            trace.add_time_ms("bounded_interval",seconds*1000.0);
            trace.add_time_ms("bounded_interval_preparation",preparation_seconds*1000.0);
        )
    }
} bounded_interval_stats;
class TemporalLNS;
bool bounded_interval_neighbor(TemporalLNS&,const vector<Move>&,vector<Move>&,double);

'''
change('class TemporalLNS {',header+'class TemporalLNS {')
change('class TemporalLNS {\n','class TemporalLNS {\n    friend struct BoundedIntervalRebuilder;\n    friend struct BoundedIntervalAudit;\n')
change('            bool reorder=iteration%113==0;','''            const double bounded_allowance=0.03*max(PROGRAM_TIME_LIMIT_SEC*0.02,clk.elapsed()-lnsStart);
            const bool bounded_due=iteration%64==0;
            const bool bounded=bounded_due&&bounded_interval_stats.seconds<bounded_allowance;
            LOCAL_NOTE(bounded_interval_stats.budget_skips+=bounded_due&&!bounded;)
            bool reorder=!bounded&&iteration%113==0;''')
change('            bool paired=!reorder&&iteration%17==0&&','            bool paired=!bounded&&!reorder&&iteration%17==0&&')
change('            bool packet=!reorder&&!paired&&!packetCuts.empty()&&iteration%7==0&&','            bool packet=!bounded&&!reorder&&!paired&&!packetCuts.empty()&&iteration%7==0&&')
change('            bool dependency=!reorder&&!paired&&!packet&&++regularSelections%4==0;','            bool dependency=!bounded&&!reorder&&!paired&&!packet&&++regularSelections%4==0;')
change('            if(reorder) {\n                ++attempts;++reorderAttempts;','''            if(bounded) {
                ++attempts;
                const double begin=clk.elapsed();
                const double budget=min(PROGRAM_TIME_LIMIT_SEC*0.001,bounded_allowance-bounded_interval_stats.seconds);
                ok=bounded_interval_neighbor(*this,current,trial,min(end,begin+budget));
                bounded_interval_stats.seconds+=clk.elapsed()-begin;
            }else if(reorder) {
                ++attempts;++reorderAttempts;''')
change('                ++accepted;uphill+=delta>0;blockAccepted+=packet;flexibleAccepted+=flexible;pairedAccepted+=paired;','''                LOCAL_NOTE(if(bounded){
                    ++bounded_interval_stats.accepted;
                    bounded_interval_stats.accepted_equal+=delta==0;
                    bounded_interval_stats.accepted_outside+=bounded_interval_stats.last_outside;
                })
                ++accepted;uphill+=delta>0;blockAccepted+=packet;flexibleAccepted+=flexible;pairedAccepted+=paired;''')
change('                    int saved=int(best.size()-polished.size());','''                    int saved=int(best.size()-polished.size());
                    LOCAL_NOTE(if(bounded){++bounded_interval_stats.best_updates;bounded_interval_stats.best_saved+=saved;})''')
change('trace.count_by("lns_regular_saved", !packet && !reorder && !paired ? saved : 0);','trace.count_by("lns_regular_saved", !bounded && !packet && !reorder && !paired ? saved : 0);')

probe=(root/'adhoc/bin/v061_bounded_lns_probe.cpp').read_text()
body=probe[probe.index('    bool insertAt('):probe.index('    static void writePlan(')]
# v061 transitions are unchanged. Local flight identities must not overwrite
# the enclosing LNS plan's metadata; only its scratch arrays are shared.
body=body.replace('lns.flights','window_flights').replace('throw StateCap{}','throw BoundedIntervalStateCap{}')
for field in ['insert_calls','links','scanned','repaired']:
 body=body.replace(f'++stats.{field};',f'LOCAL_NOTE(++stats.{field};)')
body=body.replace('stats.scanned+=t<int(base.size());','LOCAL_NOTE(stats.scanned+=t<int(base.size());)')
body=body.replace('if((t&63)==0)clk.check();LOCAL_NOTE(++stats.scanned;)',
                  'if((t&63)==0)clk.check();\n            LOCAL_NOTE(++stats.scanned;)')
body=body.replace('        for(auto m:moves) {\n            TemporalLNS::Flight f;',
                  '        for(auto m:moves) {\n            clk.check();\n            TemporalLNS::Flight f;')
# Candidate preparation counts toward the per-call deadline, including all
# fixed-background towers and each set's removable-command count.
body=body.replace('for(int p=0;p<geo.V;p++)add(tower(p));',
                  'for(int p=0;p<geo.V;p++){if((p&31)==0)clk.check();add(tower(p));}')
body=body.replace('        for(auto m:segment) {\n            int p=',
                  '        for(auto m:segment) {\n            clk.check();\n            int p=')
body=body.replace('        for(const auto& ids:bank) {\n            int saved=',
                  '        int ranked_count=0;\n        for(const auto& ids:bank) {\n            if((ranked_count++&15)==0)clk.check();\n            int saved=')
body=body.replace('        vector<vector<int>> result;for(int i=0;i<min(32,int(ranked.size()));i++)',
                  '        clk.check();\n        vector<vector<int>> result;for(int i=0;i<min(32,int(ranked.size()));i++)')

core='''// v065 implementation: each completed interval restores the full endpoint.
struct BoundedIntervalStateCap {};
struct BoundedIntervalRebuilder {
    TemporalLNS& lns;
    BoundedIntervalStats& stats=bounded_interval_stats;
    vector<TemporalLNS::Flight> window_flights;
    bool deadline_reached=false;
    explicit BoundedIntervalRebuilder(TemporalLNS& engine):lns(engine){}
'''+body+'''
    bool select(const vector<Move>& plan,uint64_t serial,int& begin,int& end,
                IdentityBoard& entry,IdentityBoard& exit) {
        if(plan.size()<2){LOCAL_NOTE(++stats.no_window;)return false;}
        const int kind=int(serial%3),span=min(int(plan.size()),16<<kind);
        // Counter hashing distributes starts without consuming the parent's
        // random stream. The three requested lengths receive equal turns.
        uint64_t cursor=(serial/3+1)*0x9e3779b97f4a7c15ULL;
        cursor=(cursor^(cursor>>30))*0xbf58476d1ce4e5b9ULL;
        cursor=(cursor^(cursor>>27))*0x94d049bb133111ebULL;
        cursor^=cursor>>31;
        begin=int(cursor%uint64_t(plan.size()-span+1));end=begin+span;
        entry=lns.checkpoints[begin/TemporalLNS::CHECKPOINT];
        for(int t=begin/TemporalLNS::CHECKPOINT*TemporalLNS::CHECKPOINT;t<begin;t++)entry.apply(plan[t]);
        exit=entry;for(int t=begin;t<end;t++){clk.check();exit.apply(plan[t]);}
        stats.last_begin=begin;stats.last_span=span;
        LOCAL_NOTE(++stats.windows;
            stats.windows_16+=kind==0;stats.windows_32+=kind==1;stats.windows_64+=kind==2;
            stats.max_span=max<int64_t>(stats.max_span,span);)
        return true;
    }

    bool rebuild(const IdentityBoard& entry,const IdentityBoard& exit,const vector<Move>& original,
                 const vector<int>& ids,vector<Move>& result) {
        bool found=false;
        try {
            clk.check();LOCAL_NOTE(++stats.sets_tried;)
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
                        if(!same(replay(restored,next),target))throw logic_error("bounded reinsertion endpoint mismatch");
                        tail=move(next);
                    }
                }catch(const BoundedIntervalStateCap&){LOCAL_NOTE(++stats.state_caps;)complete=false;}
                if(!complete)continue;
                if(!same(replay(words(entry),tail),words(exit)))throw logic_error("bounded interval endpoint mismatch");
                LOCAL_NOTE(++stats.completed;stats.equal+=tail.size()==original.size();stats.shorter+=tail.size()<original.size();)
                if(!TemporalLNS::sameMoves(tail,original)&&(!found||tail.size()<result.size())){
                    result=move(tail);found=true;
                }
            }while(next_permutation(order.begin(),order.end()));
        }catch(const Deadline&){deadline_reached=true;LOCAL_NOTE(++stats.deadlines;)}
        return found;
    }
    static bool usesOutside(const vector<Move>& original,const vector<Move>& changed) {
        array<bool,MV> touched{};
        for(Move m:original)touched[m.p]=touched[geo.ray[m.p][m.d][m.l]]=true;
        for(Move m:changed)if(!touched[m.p]||!touched[geo.ray[m.p][m.d][m.l]])return true;
        return false;
    }
    bool run(const vector<Move>& plan,vector<Move>& result,uint64_t serial) {
        int begin=0,end=0;IdentityBoard entry,exit;vector<Move> original;
        vector<vector<int>> sets;
        {
            LOCAL_NOTE(struct PreparationTimer {
                double start=clk.elapsed();
                ~PreparationTimer(){bounded_interval_stats.preparation_seconds+=clk.elapsed()-start;}
            } preparation_timer;)
            if(!select(plan,serial,begin,end,entry,exit))return false;
            original.assign(plan.begin()+begin,plan.begin()+end);
            sets=candidates(entry,original);
            stats.last_candidates=int(sets.size());LOCAL_NOTE(stats.candidates+=sets.size();)
        }
        vector<Move> best;bool found=false;
        try {
            for(const auto& ids:sets) {
                clk.check();vector<Move> trial;
                if(rebuild(entry,exit,original,ids,trial)&&(!found||trial.size()<best.size())) {
                    best=move(trial);found=true;
                }
                if(deadline_reached)break;
            }
        }catch(const Deadline&){LOCAL_NOTE(++stats.deadlines;)}
        if(!found)return false;
        result.clear();result.reserve(plan.size()-original.size()+best.size());
        result.insert(result.end(),plan.begin(),plan.begin()+begin);
        result.insert(result.end(),best.begin(),best.end());
        result.insert(result.end(),plan.begin()+end,plan.end());
        stats.last_saved=int(original.size())-int(best.size());
        LOCAL_NOTE(stats.last_outside=usesOutside(original,best);
            ++stats.returned;stats.returned_saved+=stats.last_saved;
            stats.returned_outside+=stats.last_outside;
            stats.outside_saved+=stats.last_outside?stats.last_saved:0;
            stats.max_returned_span=max<int64_t>(stats.max_returned_span,stats.last_span);)
        return true;
    }
};
bool bounded_interval_neighbor(TemporalLNS& lns,const vector<Move>& plan,vector<Move>& result,double deadline) {
    auto& stats=bounded_interval_stats;LOCAL_NOTE(++stats.calls;)
    stats.last_span=stats.last_saved=stats.last_begin=stats.last_candidates=0;stats.last_outside=false;
    const uint64_t serial=stats.serial++;
    // Restore the enclosing search's deadline even if preparation times out.
    struct LimitGuard {double old=clk.limit;~LimitGuard(){clk.limit=old;}} guard;
    clk.limit=min(clk.limit,deadline);
    try {
        clk.check();BoundedIntervalRebuilder rebuild(lns);
        return rebuild.run(plan,result,serial);
    }catch(const Deadline&){LOCAL_NOTE(++stats.deadlines;)return false;}
}
// End v065 implementation.

'''
change('#ifdef LOCAL\n// Replay using individual identities',core+'#ifdef LOCAL\n// Replay using individual identities')
change('        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();','        local_dependency.summary();local_late_start.summary();local_two_order.summary();search_reductions.summary();\n        bounded_interval_stats.summary();')

out=root/'src/bin/v065_bounded_interval_lns.cpp'
audit=root/'adhoc/v065_audit';audit.mkdir(exist_ok=True)
assert not (audit/'20260930_submit/manifest.json').exists(),'実行前に固定した実装を上書きしない'
out.write_text(source)
(audit/'registered_changes.json').write_text(json.dumps(changes,ensure_ascii=False,indent=2)+'\n')
restored=source
for c in reversed(changes):
 assert restored.count(c['new'])==1,c['new'][:100]
 restored=restored.replace(c['new'],c['old'],1)
assert restored==original
print('Created',out,'bytes',len(source.encode()),'registered edits',len(changes))
