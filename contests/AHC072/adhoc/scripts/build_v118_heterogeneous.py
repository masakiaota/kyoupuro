#!/usr/bin/env python3
"""v115の4枠のうち1枠へ、v214の完成した初期計画を移植する。"""
from pathlib import Path
from check_v113_integrated import block
from v089_data import ROOT, save, sha, now

RUN = ROOT/'results/nn_rank/v118/20261004_heterogeneous_studio'
CONDITIONS = {'mixed': 0}
BASE = ROOT/'src/bin/v115_corridor_reinsertion.cpp'
CLASSIC = ROOT/'src/bin/v214_local_color_runs.cpp'

EXTRA = r'''
struct HeterogeneousStats {
    int tree_attempts=0,tree_completed=0,merge_attempts=0,merge_completed=0;
    int deadlines=0,skipped=0,retained=0,best_ops=0,best_kind=-1;
    double seconds=0;
    void summary() const {
        LOCAL_ONLY(
            auto count=[&](const char* key,int value){trace.count_by(string("heterogeneous_")+key,value);};
            count("tree_attempts",tree_attempts);count("tree_completed",tree_completed);
            count("merge_attempts",merge_attempts);count("merge_completed",merge_completed);
            count("deadlines",deadlines);count("skipped",skipped);count("retained",retained);
            count("best_ops",best_ops);count("best_kind",best_kind);
            trace.add_time_ms("heterogeneous",seconds*1000.0);
        );
    }
} heterogeneous_stats;

#ifdef V118_SEED_DUMP
// 機構確認だけで各完成候補を独立再生する。提出ビルドには含まれない。
static void dump_v118_seed(const vector<Move>& moves,int origin) {
    const char* directory=getenv("V118_SEED_DIRECTORY");
    if(!directory)throw logic_error("missing seed diagnostic directory");
    ofstream out(string(directory)+"/seed_"+to_string(origin)+".txt");
    if(!out)throw logic_error("seed diagnostic output");
    for(auto m:moves)out<<board_info.row[m.p]<<' '<<board_info.col[m.p]<<' '<<int(m.k)<<' '
        <<board_info.dirs[m.d]<<' '<<int(m.l)<<'\n';
}
#endif
'''

START = r'''
static void classical_start(InitialSolutions& pool,vector<Move>& best) {
    const double start=time_keeper.exact_elapsed_sec();
    const double deadline=min(0.30*PROGRAM_TIME_LIMIT_SEC,start+0.10*PROGRAM_TIME_LIMIT_SEC);
    if(deadline-start<0.01*PROGRAM_TIME_LIMIT_SEC){++heterogeneous_stats.skipped;return;}
    // 古典構築だけで乱数と短い締切を所有し、後続のNN・LNSへ消費を持ち越さない。
    struct Scope {
        uint64_t random=rng.x;
        double limit=time_keeper.time_limit_sec;
        ~Scope(){rng.x=random;time_keeper.time_limit_sec=limit;}
    } scope;
    time_keeper.time_limit_sec=min(scope.limit,deadline);
    vector<Move> selected;
    auto offer=[&](const ConstructionState& proposal,int kind) {
        if(proposal.E)throw logic_error("incomplete classical proposal");
#ifdef V118_SEED_DUMP
        dump_v118_seed(proposal.moves,-1180-kind);
#endif
        if(selected.empty()||proposal.moves.size()<selected.size()) {
            selected=proposal.moves;heterogeneous_stats.best_kind=kind;
        }
    };
    try {
        for(int variant=0;variant<3;++variant) {
            time_keeper.check();++heterogeneous_stats.tree_attempts;
            auto proposal=tree_baseline(variant);++heterogeneous_stats.tree_completed;
            offer(proposal,variant);
        }
        time_keeper.check();++heterogeneous_stats.merge_attempts;
        EmptyDP potential;
        Constructor solver(potential,0,deadline,selected.size()+1);
        auto proposal=solver.run();++heterogeneous_stats.merge_completed;
        offer(proposal,3);
    } catch(const Deadline&) {
        // 未完成列は破棄する。完成した候補どうしだけを同じ育成枠で競わせる。
        ++heterogeneous_stats.deadlines;
    }
    if(!selected.empty()) {
        heterogeneous_stats.best_ops=int(selected.size());
        pool.offer(selected,-118);heterogeneous_stats.retained=1;
        if(selected.size()<best.size())best=move(selected);
    }
    heterogeneous_stats.seconds=time_keeper.exact_elapsed_sec()-start;
}
'''

def build():
    assert sha(BASE)=='00c649abb06fa8f46927ba50e5f3b7f000c806c0029bbbec683e79712d04387e'
    assert sha(CLASSIC)=='f441add3a6295c6344897ed4969d64f98d5e039b141e1d6e7765ab25fccabd9e'
    text=BASE.read_text();original=CLASSIC.read_text()
    a=original.index('struct ConstructionState {');b=original.index('// 塔の上側だけを運ぶ最短経路。',a)
    copied=original[a:b]
    singles=block(copied,'void finish_singles(')
    copied=copied.replace('// 期限付近の1匹ずつの帰巣。高さ8の塔を先に空け、通過先の容量を確保する。\n'+singles+'\n','')
    fallback='            if(time_keeper.exact_elapsed_sec()>LOCAL_SECONDS(1.80)) {finish_singles(w);break;}\n'
    assert copied.count(fallback)==1;copied=copied.replace(fallback,'')
    # 木構築にも共通の締切を適用する。期限なし診断では操作と乱数を変えない。
    copied=copied.replace('void move(Move m) { E-=','void move(Move m) { time_keeper.check(); E-=',1)
    assert 'finish_singles' not in copied
    anchor='class PortionRouter {'
    assert text.count(anchor)==1
    text=text.replace(anchor,copied+'\n'+anchor,1)
    text=text.replace('struct InitialSolutions {',EXTRA+'\nstruct InitialSolutions {',1)
    text=text.replace('void offer(const vector<Move>& moves,int origin) {','void offer(const vector<Move>& moves,int origin) {\n#ifdef V118_SEED_DUMP\n        dump_v118_seed(moves,origin);\n#endif',1)
    text=text.replace('entries.size()==5','entries.size()==4',1).replace('entries.size()>5','entries.size()>4',1)
    text=text.replace('struct ReactiveRaceStats {',START+'\nstruct ReactiveRaceStats {',1)
    assert text.count('for(int sym:{4,1,5})')==1
    text=text.replace('for(int sym:{4,1,5})','for(int sym:{4,1})')
    text=text.replace('    neural_extra_starts(input,initial_solutions,best);','    classical_start(initial_solutions,best);\n    neural_extra_starts(input,initial_solutions,best);',1)
    text=text.replace('        corridor_stats.summary();','        corridor_stats.summary();heterogeneous_stats.summary();local_collection.summary();',1)
    source=ROOT/'adhoc/bin/v118_heterogeneous_initial.cpp'
    text='// '+source.name+'\n// v118: NN 3本と集荷・配送1本を同じ初期計画の育成枠で比較する。\n'+text.split('\n',1)[1]
    assert len(text.encode())<512000
    if source.exists():assert source.read_text()==text
    else:source.write_text(text)
    save(RUN/'sources.json',dict(sources={'mixed':dict(path=str(source.relative_to(ROOT)),sha256=sha(source))},
                               parents={str(p.relative_to(ROOT)):sha(p) for p in (BASE,CLASSIC)},created_at=now()))
    return source

if __name__=='__main__':print(build())
