#!/usr/bin/env python3
"""v115の修復器を保ち、実測した利得/時間で6方式を選ぶ。"""
from v089_data import ROOT, save, sha, now

RUN=ROOT/'results/nn_rank/v120/20261004_adaptive_studio'
BASE=ROOT/'src/bin/v115_corridor_reinsertion.cpp'
CONDITIONS={'adaptive':0}

POLICY=r'''
// 各初期計画の修復履歴だけを使う。完成列の比較や受理規則は実際の手数に従う。
struct RepairAllocation {
    static constexpr int kinds=6;
    static array<double,kinds> prior() {
        const double reorder=1.0/113.0,paired=(1.0-reorder)/17.0;
        const double packet=(1.0-reorder-paired)/7.0;
        const double regular=1.0-reorder-paired-packet;
        return {regular*(1.0-1.0/dependency_interval),regular/dependency_interval,
                packet/3.0,packet*2.0/3.0,paired,reorder};
    }
    array<double,kinds> reward{},seconds{},weight=prior();
    array<double,kinds> total_seconds{},minimum{1,1,1,1,1,1},maximum{};
    array<int64_t,kinds> trials{},saved{},changes{};
    int64_t observations=0,adaptive_draws=0,updates=0;
    bool ready()const{return observations>=128;}
    void observe(int kind,double elapsed,int gain,bool changed) {
        if(kind<0)return;
        elapsed=max(elapsed,PROGRAM_TIME_LIMIT_SEC*1e-7);
        const double utility=min(8,max(0,gain))+(changed?0.05:0.0);
        ++observations;++trials[kind];saved[kind]+=max(0,gain);changes[kind]+=changed;
        total_seconds[kind]+=elapsed;reward[kind]+=utility;seconds[kind]+=elapsed;
        if(observations%64)return;
        ++updates;
        for(int i=0;i<kinds;++i){reward[i]*=0.75;seconds[i]*=0.75;}
        const double rate=(accumulate(reward.begin(),reward.end(),0.0)+1.0)/
                          (accumulate(seconds.begin(),seconds.end(),0.0)+0.01*PROGRAM_TIME_LIMIT_SEC);
        const double prior_seconds=0.002*PROGRAM_TIME_LIMIT_SEC;
        const auto base=prior();array<double,kinds> evidence{};double total=0;
        for(int i=0;i<kinds;++i){
            evidence[i]=base[i]*(reward[i]+prior_seconds*rate)/(seconds[i]+prior_seconds);
            total+=evidence[i];
        }
        for(int i=0;i<kinds;++i)weight[i]=0.25*base[i]+0.75*evidence[i]/total;
    }
    array<double,kinds> probabilities(const array<bool,kinds>& allowed)const {
        array<double,kinds> p{};double sum=0;
        for(int i=0;i<kinds;++i)if(allowed[i])sum+=weight[i];
        if(!(sum>0))throw logic_error("no available repair method");
        for(int i=0;i<kinds;++i)if(allowed[i])p[i]=weight[i]/sum;
        return p;
    }
    int select(const array<bool,kinds>& allowed,double draw) {
        ++adaptive_draws;const auto p=probabilities(allowed);
        int last=-1;
        for(int i=0;i<kinds;++i)if(allowed[i]) {
            last=i;minimum[i]=min(minimum[i],p[i]);maximum[i]=max(maximum[i],p[i]);
        }
        double cumulative=0;
        for(int i=0;i<kinds;++i)if(allowed[i]) {
            cumulative+=p[i];if(draw<cumulative)return i;
        }
        // [0,1)の抽選。和の浮動小数点丸め分は最後の合法な区間へ含める。
        return last;
    }
    struct Observation {
        RepairAllocation& owner;int& kind;const vector<Move>& best;const int& accepted;
        double begin;int previous_best,previous_accepted;
        Observation(RepairAllocation& a,int& k,const vector<Move>& b,const int& n):
            owner(a),kind(k),best(b),accepted(n),begin(time_keeper.exact_elapsed_sec()),
            previous_best(int(b.size())),previous_accepted(n){}
        ~Observation(){owner.observe(kind,time_keeper.exact_elapsed_sec()-begin,
                                    previous_best-int(best.size()),accepted>previous_accepted);}
    };
    void add_statistics(const RepairAllocation& other) {
        observations+=other.observations;adaptive_draws+=other.adaptive_draws;updates+=other.updates;
        for(int i=0;i<kinds;++i){
            trials[i]+=other.trials[i];saved[i]+=other.saved[i];changes[i]+=other.changes[i];
            total_seconds[i]+=other.total_seconds[i];
            minimum[i]=min(minimum[i],other.minimum[i]);maximum[i]=max(maximum[i],other.maximum[i]);
        }
    }
    void summary()const {
        LOCAL_ONLY(
            trace.count_by("allocation_observations",observations);
            trace.count_by("allocation_draws",adaptive_draws);trace.count_by("allocation_updates",updates);
            for(int i=0;i<kinds;++i){
                const string key="allocation_"+to_string(i);
                trace.count_by(key+"_trials",trials[i]);trace.count_by(key+"_saved",saved[i]);
                trace.count_by(key+"_changes",changes[i]);
                trace.count_by(key+"_min_ppm",llround(minimum[i]*1e6));
                trace.count_by(key+"_max_ppm",llround(maximum[i]*1e6));
                trace.add_time_ms(key,total_seconds[i]*1000.0);
            }
        );
    }
};
'''

CHOICE=r'''
            int repair_kind=-1;
            RepairAllocation::Observation observation(allocation,repair_kind,best,accepted);
            const bool adaptive=allocation.ready();
            bool reorder=false,paired=false,packet=false,flexible=false,dependency=false;
            bool backward=false;
            const double available_seconds=max(PROGRAM_TIME_LIMIT_SEC*(0.03/1.90),
                active_seconds+time_keeper.exact_elapsed_sec()-slice_begin);
            if(adaptive) {
                const bool paired_available=paired_seconds<0.11*available_seconds;
                const bool packet_available=block_seconds<0.22*available_seconds;
                repair_kind=allocation.select({true,true,packet_available,packet_available,paired_available,true},rng.unit());
                reorder=repair_kind==5;paired=repair_kind==4;
                packet=repair_kind==2||repair_kind==3;flexible=repair_kind==3;dependency=repair_kind==1;
                if(paired||packet)ensure_packets(current);
                else if(!reorder)ensure_history(current);
                // 予定列に対象がない選択も費用を記録する。他方式の成功へ付け替えない。
                if(packet&&packet_cuts.empty())continue;
            } else {
                reorder=iteration%113==0;
                paired=!reorder&&iteration%17==0&&
                        paired_seconds<0.11*max(PROGRAM_TIME_LIMIT_SEC*(0.03/1.90),active_seconds+time_keeper.exact_elapsed_sec()-slice_begin);
                const bool packet_slot=!reorder&&!paired&&iteration%7==0&&
                        block_seconds<0.22*max(PROGRAM_TIME_LIMIT_SEC*(0.03/1.90),active_seconds+time_keeper.exact_elapsed_sec()-slice_begin);
                repair_kind=reorder?5:paired?4:packet_slot?2:0;
                if(paired||packet_slot)ensure_packets(current);
                else if(!reorder)ensure_history(current);
                packet=packet_slot&&!packet_cuts.empty();
                flexible=packet&&(rng(3)!=0);
                dependency=!reorder&&!paired&&!packet&&++regular_selections%dependency_interval==0;
                repair_kind=reorder?5:paired?4:packet?(flexible?3:2):dependency?1:0;
            }
'''

def build():
    assert sha(BASE)=='00c649abb06fa8f46927ba50e5f3b7f000c806c0029bbbec683e79712d04387e'
    text=BASE.read_text()
    assert text.count('class TemporalLNS {')==1
    text=text.replace('class TemporalLNS {',POLICY+'\nclass TemporalLNS {',1)
    text=text.replace('    uint64_t random_state=1;','    RepairAllocation allocation;\n    uint64_t random_state=1;',1)
    text=text.replace('    void accumulate(const TemporalLNS& other) {','    void accumulate(const TemporalLNS& other) {\n        allocation.add_statistics(other.allocation);',1)
    begin=text.index('            bool reorder=iteration%113==0;')
    end=text.index('            LOCAL_NOTE(DependencyTimer dependency_timer(dependency);)',begin)
    text=text[:begin]+CHOICE+text[end:]
    text=text.replace('        corridor_stats.summary();','        corridor_stats.summary();lns.allocation.summary();',1)
    source=ROOT/'adhoc/bin/v120_adaptive_repair.cpp'
    text='// '+source.name+'\n// v120: 修復方式の実測利得/費用に応じて抽選する。\n'+text.split('\n',1)[1]
    if source.exists():assert source.read_text()==text
    else:source.write_text(text)
    assert len(text.encode())<512000
    save(RUN/'sources.json',dict(sources={'adaptive':dict(path=str(source.relative_to(ROOT)),sha256=sha(source))},
                               parents={str(BASE.relative_to(ROOT)):sha(BASE)},created_at=now()))
    return source

if __name__=='__main__':print(build())
