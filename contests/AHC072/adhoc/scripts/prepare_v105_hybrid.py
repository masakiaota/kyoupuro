#!/usr/bin/env python3
"""凍結したNNとv214から、初期解だけを変更した単一C++を作る。"""
import argparse
import json
from pathlib import Path
import shutil

import torch

from build_v090_board import build
from v089_data import ROOT, save, sha, now
from v099_complete import BC_RUN


def replace_once(text, old, new):
    assert text.count(old) == 1, old[:120]
    return text.replace(old, new, 1)


def prepare(root):
    frozen = root / 'frozen'
    identity = json.loads((frozen / 'identity.json').read_text())
    assert sha(frozen / 'checkpoint.pt') == identity['checkpoint_sha256']
    assert sha(frozen / 'v214_original.cpp') == identity['traditional_sha256']
    saved = torch.load(frozen / 'checkpoint.pt', map_location='cpu', weights_only=False)
    assert saved['iteration'] == 165 and saved['steps'] == 7358474
    model = json.loads((frozen / 'model_template.json').read_text())
    model.update(parameters={k: v.tolist() for k, v in saved['model'].items()},
                 epoch=165, rl_iterations=165, environment_steps=saved['steps'],
                 initial_checkpoint_sha256=identity['checkpoint_sha256'])
    save(root / 'training/model.json', model)
    nn = ROOT / 'adhoc/bin/v105_nn.cpp'
    build(root / 'training/model.json', nn)
    nn.write_text(nn.read_text().replace('// epoch=', '// PPO iteration=', 1))
    classical = ROOT / 'adhoc/bin/v105_classical.cpp'
    original = (frozen / 'v214_original.cpp').read_text()
    classical.write_text(replace_once(original, '// v214_local_color_runs.cpp', '// v105_classical.cpp'))

    nn_text = nn.read_text()
    nn_body = nn_text[nn_text.index('struct Input {'):nn_text.index('int main(int argc,char** argv)')]
    nn_body = replace_once(nn_body,
        'SearchResult search(const Input& input,const Board& board,const BoardState& initial,TimeKeeper& timer)',
        'template<class Timer>\nSearchResult search(const Input& input,const Board& board,const BoardState& initial,Timer& timer)')
    # 初期構築を使わないため、その専用DPと構築器を残さない。
    traditional_body = original[:original.index('struct ConstructionState')]
    traditional_body += original[original.index('class PortionRouter'):original.index('int main()')]
    traditional_body = replace_once(traditional_body,
        'pool.entries[id].origin<0?0:local_collection.completed_adoptions.at(pool.entries[id].origin)', '0')
    main_tail = original[original.index('    size_t before_lns=best.size();'):]
    start = main_tail.index('    try {\n        State check=board_info.initial;')
    stop = main_tail.index('\n    cerr<<', start)
    main_tail = main_tail[:start] + '''    {
        State check=board_info.initial;
        for(auto m:best)board_info.apply(check,m);
        if(board_info.count(check)||best.size()>100000)throw logic_error("final answer check");
    }
''' + main_tail[stop:]
    assert main_tail.count('potential.memo.size()') == 2
    main_tail = main_tail.replace('potential.memo.size()', '0')
    main_tail = replace_once(main_tail, 'local_nn.summary();local_collection.summary();trace.summary();',
        'local_nn.summary();trace.count_by("E",0);trace.count_by("T",best.size());trace.summary();')

    main_head = r'''
// NNとLNSは同じ開始時刻を参照する。NNの準備と変換の費用も探索時間から引く。
struct NNSharedDeadline {
    double limit;
    bool step() const {return time_keeper.exact_elapsed_sec()<limit;}
};

int main() {
#ifdef LOCAL
    time_keeper.start_=chrono::steady_clock::now();
    for(const string key:{"finish_singles_calls","baseline_recovery","final_recovery",
        "construction_errors","lns_errors","construction_deadlines","lns_deadlines",
        "lns_initial_shortcut_saved","lns_initial_reorder_saved","lns_best_saved",
        "lns_regular_saved","lns_packet_saved","lns_invalid_candidates"})trace.count_by(key,0);
#endif
    ios::sync_with_stdio(false);cin.tie(nullptr);
    board_info.read();state_pool.prepare(board_info.cell_count);
    for(int p=0;p<board_info.cell_count;p++)
        rng.x=(rng.x^(board_info.initial[p]+17*board_info.nest_code[p]+p))*0x9e3779b97f4a7c15ULL;
    nn::load_nn();nn::Input input;input.N=board_info.N;input.K=board_info.K;input.C.resize(input.N);
    for(int i=0;i<input.N;i++)input.C[i]=board_info.C[i];
    const nn::Board board(input);const nn::BoardState initial(input,board);
#ifdef LOCAL
    NNSharedDeadline timer{PROGRAM_TIME_LIMIT_SEC};
#else
    NNSharedDeadline timer{LOCAL_SECONDS(1.900)};
#endif
    const auto result=nn::search(input,board,initial,timer);
    vector<Move> best;best.reserve(result.path.size());
    uint64_t hash=1469598103934665603ULL;
    {
        State checked=board_info.initial;
        for(nn::Move m:result.path){
            const auto [i,j]=board.coordinates[m.p()];
            Move converted{int16_t(board_info.cell_id[i][j]),uint8_t(m.k()),uint8_t(m.d()),uint8_t(m.l())};
            board_info.apply(checked,converted);best.push_back(converted);
            const uint64_t code=(((uint64_t(i*20+j)*8+m.k())*4+m.d())*8+m.l()-1);
            hash=(hash^code)*1099511628211ULL;
        }
        if(board_info.count(checked)!=result.E)throw logic_error("NN operation conversion mismatch");
    }
    LOCAL_ONLY(
        trace.count_by("initial_E",initial.E);trace.count_by("nn_initial_ops",best.size());
        trace.count_by("nn_initial_hash",hash&0x7fffffffffffffffULL);
        trace.count_by("nn_complete",result.E==0);trace.count_by("inferences",result.inferences);
        trace.count_by("depth",result.depth);trace.count_by("deadline",result.deadline);
        trace.count_by("duplicates",result.duplicates);trace.count_by("exhausted",result.exhausted);
        trace.add_time_ms("nn_initial",time_keeper.exact_elapsed_sec()*1000.0);
    );
    // NNの未完了はそのまま記録する。完成解を前提とするLNSには渡さない。
    if(result.E!=0){
        for(auto m:best)cout<<board_info.row[m.p]<<' '<<board_info.col[m.p]<<' '<<int(m.k)<<' '
            <<board_info.dirs[m.d]<<' '<<int(m.l)<<'\n';
        LOCAL_ONLY(trace.count_by("E",result.E);trace.count_by("T",best.size());trace.summary(););
        return 0;
    }
    const size_t baseline=best.size();
    InitialSolutions initial_solutions;initial_solutions.offer(best,-105);
    const int completed=1,attempts=1;
'''
    hybrid = ROOT / 'src/bin/v105_nn_lns.cpp'
    prefix = '''// v105_nn_lns.cpp
// v104の固定165採取時点のNNで完成解を1本作り、v214のLNSと短縮処理へ渡す。
// 学習条件と重みの由来、時間を含む比較条件は notes/experiments/v105.md に記録する。
#include <bits/stdc++.h>
using namespace std;
#if defined(__GNUC__) && !defined(__clang__)
#pragma GCC push_options
#pragma GCC optimize("O3,unroll-loops")
#endif
namespace nn {
'''
    middle = '''} // namespace nn
#if defined(__GNUC__) && !defined(__clang__)
#pragma GCC pop_options
#endif
'''
    hybrid.write_text(prefix + nn_body + middle + traditional_body + main_head + main_tail)
    manifest = json.loads((BC_RUN / 'input_manifest.json').read_text())
    cases = [dict(c,filename=f"{c['index']:06d}.txt") for c in manifest if c['role']=='validation']
    assert len(cases)==256
    inputs=root/'inputs';inputs.mkdir(exist_ok=True)
    for case in cases:
        source=BC_RUN/case['path'];assert sha(source)==case['sha256']
        shutil.copy2(source,inputs/case['filename'])
    save(root/'input_manifest.json',cases)
    paths={'classical':classical,'nn':nn,'hybrid':hybrid}
    for source in paths.values():shutil.copy2(source,frozen/source.name)
    save(root/'config.json',dict(checkpoint=identity,iteration=165,jobs=12,cases=256,
        model_sha256=sha(root/'training/model.json'),input_sha256=sha(root/'input_manifest.json'),
        sources={k:dict(path=str(p.relative_to(ROOT)),sha256=sha(p)) for k,p in paths.items()},
        created_at=now(),clock='v214 shared clock; LOCAL 1.520/1.544 seconds'))
    print(json.dumps({'sources':{k:str(v) for k,v in paths.items()},'iteration':165},ensure_ascii=False))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);a=p.parse_args()
    torch.set_num_threads(2);prepare(a.run.resolve())
