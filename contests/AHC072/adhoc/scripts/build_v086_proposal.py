#!/usr/bin/env python3
"""凍結した集合生成モデルを、提出できる単独C++へ組み込む。"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT = ROOT / 'results/nn_rank/v086/20261002T181701_studio/model.json'


def literal(values):
    if isinstance(values, list):
        return '{' + ','.join(literal(v) for v in values) + '}'
    return float(values).hex() + 'f'


def declaration(name, values):
    shape = ''
    part = values
    while isinstance(part, list):
        shape += f'[{len(part)}]'
        part = part[0]
    return f'constexpr float {name}{shape}={literal(values)};\n'


def build(model_path, output):
    model = json.loads(model_path.read_text())
    weights = declaration('mean', model['mean']) + declaration('scale', model['scale'])
    for name, value in model['parameters'].items():
        weights += declaration(name.replace('.', '_'), value)
    source = (ROOT / 'src/bin/v079_nn_immediate.cpp').read_text()
    support = (ROOT / 'adhoc/scripts/v086_proposal.cpp.txt').read_text()
    namespace, members = support.split('// V086_MEMBERS\n')
    namespace = namespace.replace('// V086_WEIGHTS', weights)
    feature = (ROOT / 'adhoc/scripts/v084_set_support.cpp.txt').read_text()
    feature = feature[feature.index('    void prepare_nn084_items'):feature.index('    pair<array<float,8>')]
    feature = '    bool nn086_items_ready=false;\n    array<array<double,20>,max_cells> nn086_items{};\n' + feature.replace('nn084', 'nn086')
    members = members.replace('    // V086_ITEM_FEATURES', feature)
    source = source.replace('class TemporalLNS {', namespace + '\nclass TemporalLNS {', 1)
    source = source.replace('    int nn_choose(const vector<Move>& current', members + '\n    int nn_choose(const vector<Move>& current', 1)
    source = source.replace('    void make_candidates(const vector<Move>& answer) {',
                            '    void make_candidates(const vector<Move>& answer) {\n        nn086_ready=false;nn086_items_ready=false;', 1)
    assert 'nn086_ready=false;nn086_items_ready=false;' in source
    source = source.replace('            bool ok=true;\n            LOCAL_NOTE(int late_kind=',
                            '            bool ok=true,generated=false;\n            LOCAL_NOTE(int late_kind=', 1)
    start = source.index('                int choice=-1,cooldown=max(24,min(200,')
    end = source.index('                LOCAL_NOTE(repair_priority_broken=', start)
    original = source[start:end]
    original = original.replace('int choice=-1,cooldown=max(24,min(200,int(candidates.size()*regular_cooldown_fraction)));', 'int choice=-1;')
    original = original.replace('Removal cand=candidates[choice];last_tried[cand.hash]=iteration;', 'cand=candidates[choice];')
    replacement = '''                const int cooldown=max(24,min(200,int(candidates.size()*regular_cooldown_fraction)));
                Removal cand;
                // 通常近傍の4回に1回だけ、固定温度1の生成集合を既存の再構築へ渡す。
                generated=!dependency&&++nn086_regular%4==0;
                if(generated) {
                    cand=nn086_propose(current);
                    auto previous=last_tried.find(cand.hash);
                    if(previous!=last_tried.end()&&iteration-previous->second<=cooldown) {
                        LOCAL_NOTE(trace.count_by("nn086_cooldown_skips",1);)
                        continue;
                    }
                    LOCAL_NOTE(trace.count_by("nn086_attempts",1);)
                }else {
''' + original + '''                }
                last_tried[cand.hash]=iteration;
'''
    source = source[:start] + replacement + source[end:]
    source = source.replace('                if(ok){trial=move(base);LOCAL_NOTE(local_dependency.completed+=dependency;)}',
                            '                if(ok){trial=move(base);LOCAL_NOTE(local_dependency.completed+=dependency;trace.count_by("nn086_rebuilt",generated);)}', 1)
    source = source.replace('            int delta=int(polished.size())-int(current.size());',
                            '            int delta=int(polished.size())-int(current.size());\n            LOCAL_NOTE(if(generated){trace.count_by("nn086_current_saved",max(0,-delta));trace.count_by("nn086_current_improvements",delta<0);})', 1)
    source = source.replace('            if(accept) {\n',
                            '            if(accept) {\n                LOCAL_NOTE(trace.count_by("nn086_accepted",generated);)\n', 1)
    source = source.replace('                    int saved=int(best.size()-polished.size());',
                            '                    int saved=int(best.size()-polished.size());\n                    LOCAL_NOTE(if(generated){trace.count_by("nn086_best_saved",saved);trace.count_by("nn086_best_improvements",1);})', 1)
    source = '// ' + output.name + '\n' + source.split('\n', 1)[1]
    assert '#include "' not in source
    output.write_text(source)


if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--model',type=Path,default=DEFAULT)
    parser.add_argument('--output',type=Path,default=ROOT/'src/bin/v086_nn_proposal.cpp')
    args=parser.parse_args();build(args.model,args.output)
