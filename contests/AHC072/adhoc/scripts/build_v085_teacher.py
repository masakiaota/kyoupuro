#!/usr/bin/env python3
"""凍結したv800/v079へ、有界な成功経路の採取と長時間入口を加える。"""
from pathlib import Path
import hashlib
import re

ROOT = Path(__file__).resolve().parents[2]
base800 = (ROOT/'adhoc/bin/v800.cpp').read_text()
base079 = (ROOT/'src/bin/v079_nn_immediate.cpp').read_text()
assert hashlib.sha256(base800.encode()).hexdigest() == '974610eec5377d946857c10b7b1898dfc332f66dc9860e5ab4abe7109c57e99d'
assert hashlib.sha256(base079.encode()).hexdigest() == 'a58af5f053bfbf6ce33463c60e21d0bbf6f659479c6aadb5d452d158da65bb8a'
support = (ROOT/'adhoc/scripts/v085_teacher_support.cpp.txt').read_text()
recording = base800[base800.index('// Saved plans,'):base800.index('\nclass TemporalLNS {')]
options = base800[base800.index('struct Options {'):base800.index('\nint main(int argc,char** argv)')]

def replace(s, old, new, count=1):
    assert s.count(old) == count, (old[:120], s.count(old), count)
    return s.replace(old, new)


def translate(s):
    names = {'geo.V':'board_info.cell_count','geo.grid':'board_info.C','geo.id':'board_info.cell_id',
             'geo.nest':'board_info.nest_code','geo.home':'board_info.nest_pos_by_code','geo.dist':'board_info.floor_dist',
             'geo.':'board_info.','clk.elapsed()':'time_keeper.exact_elapsed_sec()'}
    for old,new in names.items():
        s=s.replace(old,new)
    s=re.sub(r'\bMV\b','max_cells',s)
    s=re.sub(r'\bWord\b','TowerBits',s)
    return s


def instrument(s, modern):
    naming = {'last':'last_tried' if modern else 'lastTried',
              'tie':'tie_mode' if modern else 'tieMode',
              'keepP':'keep_p' if modern else 'keepP', 'keepQ':'keep_q' if modern else 'keepQ'}
    s=replace(s,'class TemporalLNS {',support+'\nclass TemporalLNS {\n    TeacherChoice teacher_choice;')
    s=replace(s,f'            int {naming["tie"]}=rng(12);',f'            teacher_choice.reset();teacher_choice.rng_start=rng.x;\n            int {naming["tie"]}=rng(12);')
    ride='ride_weight' if modern else 'rideWeight'
    support_weight='support_weight' if modern else 'supportWeight'
    old=f'            {ride}={naming["tie"]}==3?4:1;'
    s=replace(s,old,old+f'\n            teacher_choice.support_weight={support_weight};teacher_choice.ride_weight={ride};teacher_choice.temperature=temperature;')
    s=replace(s,'                current=best;stagnant=0;++restarts;rebuild();','                teacher_episodes.reset();\n                current=best;stagnant=0;++restarts;rebuild();')
    old=f'                Removal cand=candidates[choice];{naming["last"]}[cand.hash]=iteration;'
    s=replace(s,old,old+'\n                teacher_choice.rank=choice;teacher_choice.candidate_hash=cand.hash;')
    old='                vector<Move> base;'+('State' if modern else 'Board')+' initial;'
    s=replace(s,old,'                teacher_choice.ids=cand.ids;teacher_choice.candidate_hash=cand.hash;\n'+old)
    s=replace(s,'                sort(order.begin(),order.end());','                sort(order.begin(),order.end());\n                for(const auto& entry:order)teacher_choice.insertion_order.push_back(entry.second);')
    s=replace(s,'                int style=rng(6);backward=style>=3;', '                int style=rng(6);teacher_choice.reorder_style=style;backward=style>=3;')
    packet='packet_cuts' if modern else 'packetCuts'
    old=f'                PacketCut cut={packet}[choice];{naming["last"]}[cut.hash^salt]=iteration;'
    s=replace(s,old,old+'''
                teacher_choice.rank=choice;teacher_choice.candidate_hash=cut.hash;
                teacher_choice.packet_time=cut.time;teacher_choice.packet_keep=cut.keep;teacher_choice.packet_size=cut.size;
                teacher_choice.packet_later=cut.later;teacher_choice.packet_before_split=cut.before_split;''')
    s=replace(s,'        int source=current[cut.time].p;', '''        int source=current[cut.time].p;
        teacher_choice.ids.clear();
        for(int i=cut.keep;i<cur.h[source];i++)teacher_choice.ids.push_back(cur.a[source][i]);''',2)
    old=f'        if({naming["keepQ"]}<0)return false;'
    s=replace(s,old,old+f'''
        teacher_choice.paired_time=time;teacher_choice.paired_p=p;teacher_choice.paired_q=q;
        teacher_choice.paired_keep_p={naming['keepP']};teacher_choice.paired_keep_q={naming['keepQ']};''')
    s=replace(s,'                int id=cur.a[cell][k];','                int id=cur.a[cell][k];teacher_choice.ids.push_back(id);')
    kind='reorder?(backward?"reorder_backward":"reorder_forward"):paired?"paired":packet?(flexible?"flexible_packet":"packet"):dependency?"dependency":"regular"'
    s=replace(s,'            if(accept) {','            if(accept) {\n                teacher_episodes.observe('+kind+',current,polished,best.size(),iteration,teacher_choice,rng.x);')
    return s


# 古い探索本体は採取のフック以外を変更しない。
s800=replace(base800,'// v800.cpp','// collect_v085_teacher.cpp')
s800=instrument(s800,False)
old='        SearchRecorder log(options.output,options.progress_seconds,options.seed,budget,options.initial.string());'
s800=replace(s800,old,old+'\n        teacher_episodes.configure(options.output,options.seed,"v800");')
s800=replace(s800,'        log.complete(status,lns.attempts,lns.improvements);','        log.complete(status,lns.attempts,lns.improvements);\n        teacher_episodes.save(status,rng.x,0,0,0,0,0,false,false);')

# 最新の探索本体は時間軸だけを長時間の呼出しに接続する。
s=replace(base079,'// v079_nn_immediate.cpp','// v801_teacher.cpp')
s=replace(s,'#include <bits/stdc++.h>','#include <bits/stdc++.h>\n#include <csignal>\n#include <sys/resource.h>')
s=replace(s,'struct Deadline {};','''volatile sig_atomic_t stop_requested=0;
void request_stop(int){stop_requested=1;}
struct StopRequested {};
struct Deadline {};''')
s=replace(s,'    void check() const { if (exact_elapsed_sec() > time_limit_sec) throw Deadline(); }','    void check() const { if(stop_requested)throw StopRequested();if (exact_elapsed_sec() > time_limit_sec) throw Deadline(); }')
s=replace(s,'namespace neural_rank {','uint64_t teacher_nn_calls=0,teacher_reduction_candidates=0,teacher_reduction_shortened=0;\nnamespace neural_rank {')
s=replace(s,'        const int n=nn_sample(cooldown);','        ++teacher_nn_calls;\n        const int n=nn_sample(cooldown);')
s=replace(s,'class TemporalLNS {',translate(recording)+'\nclass TemporalLNS {')
s=replace(s,'    void optimize(vector<Move>& best) {','    void optimize(vector<Move>& best,double end,SearchRecorder& log) {')
s=replace(s,'        const double end=LOCAL_SECONDS(1.900);','        // endは呼出しごとの長時間期限。局所処理のPROGRAM比率は変更しない。')
s=replace(s,'search_reductions.cheap(current,PROGRAM_TIME_LIMIT_SEC);','search_reductions.cheap(current,end);')
s=replace(s,'search_reductions.cheap(polished,PROGRAM_TIME_LIMIT_SEC);','search_reductions.cheap(polished,end);')
s=replace(s,'        if(current.size()<best.size())best=current;','        if(current.size()<best.size()){log.improve(current,"initial_smoothing",iteration,attempts,&best);best=current;}')
s=replace(s,'                current=move(polished);if(current.size()<best.size())best=current;','                log.improve(polished,"initial_reorder",iteration,attempts,&current);\n                current=move(polished);if(current.size()<best.size())best=current;')
s=replace(s,'            time_keeper.check();++iteration;++stagnant;', '''            time_keeper.check();++iteration;++stagnant;
            // 参照される待ち時間の上限200を超えた履歴だけ除き、長時間時の蓄積を抑える。
            if((iteration&4095)==0)for(auto it=last_tried.begin();it!=last_tried.end();){
                if(iteration-it->second>200)it=last_tried.erase(it);else ++it;
            }
            if((iteration&255)==1)log.progress(iteration,attempts,accepted,int(current.size()),temperature,last_tried.size());''')
s=replace(s,'            search_reductions.candidate(polished,current,best.size(),iteration,lns_start,end);','''            const size_t teacher_before_reductions=polished.size();
            search_reductions.candidate(polished,current,best.size(),iteration,lns_start,end);
            ++teacher_reduction_candidates;teacher_reduction_shortened+=polished.size()<teacher_before_reductions;''')
s=replace(s,'                    best=polished;stagnant=0;', '''                    const char* kind=reorder?(backward?"reorder_backward":"reorder_forward"):
                        paired?"paired":packet?(flexible?"flexible_packet":"packet"):dependency?"dependency":"regular";
                    log.improve(polished,kind,iteration,attempts,&current);
                    best=polished;stagnant=0;''')
s=replace(s,'    double paired_seconds=0;','    int64_t iterations()const{return iteration;}\n    double paired_seconds=0;')
s=instrument(s,True)
s=s[:s.index('\nint main() {')]+ '\n'+options+'''
int main(int argc,char** argv){
    time_keeper.start_=chrono::steady_clock::now();
    signal(SIGINT,request_stop);signal(SIGTERM,request_stop);
    ios::sync_with_stdio(false);cin.tie(nullptr);
    try{
        const Options options=parse_options(argc,argv);
        const double budget=options.seconds,search_end=budget*0.995,finish_end=budget*0.999;
        time_keeper.time_limit_sec=search_end;
        board_info.read();state_pool.prepare(board_info.cell_count);
        if(board_info.cell_count<2*board_info.K)throw runtime_error("too few floor cells");
        for(int p=0;p<board_info.cell_count;p++){
            if(board_info.floor_dist[0][p]==infinite_cost)throw runtime_error("disconnected floor");
            rng.x=(rng.x^(board_info.initial[p]+17*board_info.nest_code[p]+p))*0x9e3779b97f4a7c15ULL;
        }
        rng.x=seed_mix(rng.x^seed_mix(options.seed));if(!rng.x)rng.x=1;
        vector<Move> best=load_plan(options.initial);
        SearchRecorder log(options.output,options.progress_seconds,options.seed,budget,options.initial.string());
        teacher_episodes.configure(options.output,options.seed,"v801");
        log.improve(best,"saved_seed",0,0);
        TemporalLNS lns;string status="completed";
        try{lns.optimize(best,search_end,log);}
        catch(const Deadline&){status="time_limit";}
        catch(const StopRequested&){status="interrupted";}
        if(stop_requested)status="interrupted";
        if(status!="interrupted"){
            time_keeper.time_limit_sec=finish_end;
            auto shortened=compress_pair_transfers(best);
            log.improve(shortened,"final_pair_compression",lns.iterations(),lns.attempts,&best);best=move(shortened);
            JointWindowStats joint_stats;FinalReductionStats final_stats;
            try{
                auto joint=compress_joint_windows(best,finish_end,joint_stats);
                log.improve(joint,"final_joint_windows",lns.iterations(),lns.attempts,&best);best=move(joint);
                auto final_plan=best;polish_final_reductions(final_plan,final_stats);
                log.improve(final_plan,"final_reductions",lns.iterations(),lns.attempts,&best);best=move(final_plan);
            }catch(const Deadline&){status="time_limit";}
             catch(const StopRequested&){status="interrupted";}
        }
        verify_plan(best);save_moves(options.output/"best.txt",best);
        if(state_pool.free_count!=4)throw logic_error("teacher leaked state pool");
        log.complete(status,lns.attempts,lns.improvements);
        uint64_t reduced=0;for(auto value:search_reductions.saved)reduced+=value;
        teacher_episodes.save(status,rng.x,teacher_nn_calls,teacher_reduction_candidates,
            search_reductions.heavy_calls,teacher_reduction_shortened,reduced,true,true);
        LOCAL_NOTE(trace.count_by("T",best.size());trace.count_by("E",0);trace.count_by("lns_accepted",lns.accepted);
                   trace.count_by("board_pool_free_at_end",state_pool.free_count);local_nn.summary();search_reductions.summary();trace.summary();)
        write_moves(cout,best);cout.flush();if(!cout)throw runtime_error("stdout flush failed");
        return status=="interrupted"?130:0;
    }catch(const exception& error){cerr<<"v801: "<<error.what()<<'\\n';return 2;}
}
'''
for name,source in [('collect_v085_teacher',s800),('v801_teacher',s)]:
    source=replace(source,'int main(int argc,char** argv)', '#ifndef TEACHER_RECORD_SELF_TEST\nint main(int argc,char** argv)')
    test=(ROOT/'adhoc/scripts/v085_recorder_test.cpp.txt').read_text()
    source+='\n#else\n'+(translate(test) if name=='v801_teacher' else test)+'\n#endif\n'
    output=ROOT/'adhoc/bin'/f'{name}.cpp';output.write_text(source);print(output)
