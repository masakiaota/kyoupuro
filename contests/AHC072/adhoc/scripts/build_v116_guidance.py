#!/usr/bin/env python3
"""固定したv111へ、有限区間の残り費用の学習評価だけを組み込む。"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT / 'src/bin/v111_weight_portfolio.cpp'
PARENT_SHA = 'dd45427b0e3e791bf40c1a7f7560c734abde1d20452ffa6c34cd878fd8c02316'
RUN = ROOT / 'results/nn_rank/v116/20261004_finite_value_studio'


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def replace_once(text, before, after):
    assert text.count(before) == 1, (before[:100], text.count(before))
    return text.replace(before, after, 1)


def block(text, marker):
    start = text.index(marker)
    a = text.index('{', start)
    depth = 1
    end = a + 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return text[start:end]


def literal(values):
    if isinstance(values, list):
        return '{' + ','.join(literal(x) for x in values) + '}'
    return float(values).hex() + 'f'


def build(output, model=None):
    assert sha(PARENT) == PARENT_SHA
    text = PARENT.read_text()
    network = (ROOT / 'adhoc/bin/v116_value.hpp').read_text()
    methods = (ROOT / 'adhoc/bin/v116_features.hpp').read_text()
    if model:
        d = json.loads(Path(model).read_text())
        weights = d['cpp']
        declarations = '\n'.join([
            'static constexpr float w1[32][48]=' + literal(weights['w1']) + ';',
            'static constexpr float b1[32]=' + literal(weights['b1']) + ';',
            'static constexpr float w2[32]=' + literal(weights['w2']) + ';',
            'static constexpr float b2=' + literal(weights['b2']) + ';',
        ])
    else:
        declarations = ('static constexpr float w1[32][48]={};\n'
                        'static constexpr float b1[32]={},w2[32]={},b2=0;')
    network = network.replace('/* V116_WEIGHTS */', declarations)
    text = replace_once(text, 'class FinitePlanner {', network + '\nclass FinitePlanner {')
    text = replace_once(text, '    int jump=1;\n', '    int jump=1;\n' + methods + '\n')
    text = replace_once(text, '        estimate=lower+0.5*distance/jump+0.125*unsettled;\n',
                        '''        estimate=lower+0.5*distance/jump+0.125*unsettled;
        // The learned value only orders the beam. All proof-based pruning uses lower.
        if(finite_value::enabled) {
            const auto features=learned_features(s,lower,estimate);
            estimate=max(double(lower),estimate+finite_value::residual(features));
        }
''')
    text = replace_once(text, '        jump=min(8,1+max_background+min(3,window.pieces-1));\n',
                        '''        jump=min(8,1+max_background+min(3,window.pieces-1));
        value_background_height.fill(0);
        for(int p=0;p<board_info.cell_count;p++)value_background_height[p]=height(background[p]);
''')
    # These hooks are absent from the submission after preprocessing.
    text = replace_once(text, '                if(expanded==192){++stats.capped;stop=true;break;}\n',
                        '''#ifdef V116_COLLECTOR
                if(expanded==finite_probe::node_limit){++stats.capped;stop=true;break;}
#else
                if(expanded==192){++stats.capped;stop=true;break;}
#endif
''')
    text = replace_once(text, '            int width=min(12,int(next.size()));stats.beam_pruned+=int(next.size())-width;\n',
                        '''#ifdef V116_COLLECTOR
            int width=min(finite_probe::width,int(next.size()));
#else
            int width=min(12,int(next.size()));
#endif
            stats.beam_pruned+=int(next.size())-width;
''')
    text = replace_once(text, '                            ++stats.generated;\n',
                        '''                            ++stats.generated;
#ifdef V116_COLLECTOR
                            if(finite_probe::edge && !finite_probe::edge(from.state,child.state,path)) {
                                stop=true;break;
                            }
#endif
''')
    text = replace_once(text, '        trace.count_by("floor_cells", board_info.cell_count);',
                        '''        trace.count_by("finite_value_calls",finite_value::calls);
        trace.count_by("floor_cells", board_info.cell_count);''')
    output = Path(output)
    text = '// ' + output.name + '\n' + text.split('\n', 1)[1]
    assert len(text.encode()) <= 512000, len(text.encode())
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text)
    return dict(source=str(output.relative_to(ROOT)), sha256=sha(output), bytes=len(text.encode()),
                parent_sha256=PARENT_SHA, model_sha256=sha(Path(model)) if model else None)


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--output', type=Path, default=ROOT/'adhoc/bin/v116_finite_value.cpp')
    p.add_argument('--model', type=Path)
    a = p.parse_args()
    print(json.dumps(build(a.output, a.model), indent=2))
