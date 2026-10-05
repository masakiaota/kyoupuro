#!/usr/bin/env python3
"""固定したv111の有限区間探索へ、小型の学習順位だけを組み込む。"""
import argparse
import json
from pathlib import Path

from build_v116_guidance import ROOT,PARENT,PARENT_SHA,sha,literal


def build(output,model=None):
    assert sha(PARENT)==PARENT_SHA
    original=PARENT.read_text();text=original;edits=[]
    def change(before,after):
        nonlocal text
        assert text.count(before)==1,(before[:80],text.count(before))
        text=text.replace(before,after,1);edits.append((before,after))
    network=(ROOT/'adhoc/bin/v116_value.hpp').read_text()
    if model:
        w=json.loads(Path(model).read_text())['cpp']
        declarations='\n'.join(['static constexpr float w1[32][48]='+literal(w['w1'])+';',
                                  'static constexpr float b1[32]='+literal(w['b1'])+';',
                                  'static constexpr float w2[32]='+literal(w['w2'])+';',
                                  'static constexpr float b2='+literal(w['b2'])+';'])
    else:
        declarations='static constexpr float w1[32][48]={};\nstatic constexpr float b1[32]={},w2[32]={},b2=0;'
    network=network.replace('/* V116_WEIGHTS */',declarations)
    methods=(ROOT/'adhoc/bin/v116_features.hpp').read_text()
    methods=methods.replace('    array<uint8_t,max_cells> value_background_height{};',
                            '    array<uint8_t,max_cells> value_background_height{};\n    int value_background_cells=0,value_background_pieces=0;')
    before='''        int background_cells=0,background_pieces=0;
        for(int p=0;p<board_info.cell_count;p++) {
            background_cells+=value_background_height[p]>0;
            background_pieces+=value_background_height[p];
        }
        f[43]=background_pieces;f[44]=background_cells;'''
    assert methods.count(before)==1
    methods=methods.replace(before,'        f[43]=value_background_pieces;f[44]=value_background_cells;')
    change('class FinitePlanner {',network+'\nclass FinitePlanner {')
    change('''        int cost=0,parent=-1,source=-1,keep=0,destination=-1,orientation=0;
        double priority=0;''','''        int cost=0,parent=-1,source=-1,keep=0,destination=-1,orientation=0;
        // Store the state value separately so a cheaper duplicate changes only g.
        double priority=0,estimate_value=0;''')
    change('    int jump=1;\n','    int jump=1;\n'+methods+'\n')
    change('        jump=min(8,1+max_background+min(3,window.pieces-1));\n','''        jump=min(8,1+max_background+min(3,window.pieces-1));
        value_background_height.fill(0);
        value_background_cells=value_background_pieces=0;
        for(int p=0;p<board_info.cell_count;p++) {
            const int h=height(background[p]);value_background_height[p]=h;
            value_background_cells+=h>0;value_background_pieces+=h;
        }
''')
    change('''                            child.priority=child.cost+estimate;
                            auto [it,fresh]=index.emplace(child.state,int(next.size()));
                            if(fresh)next.push_back(child);
                            else if(child.cost<next[it->second].cost)next[it->second]=child;''','''                            auto [it,fresh]=index.emplace(child.state,int(next.size()));
                            if(fresh) {
                                // Proof pruning and duplicate rejection happen before inference.
                                // This prediction changes beam order, never the legal moves or bound.
                                if(finite_value::enabled)
                                    estimate=max(double(lower),estimate+finite_value::residual(learned_features(child.state,lower,estimate)));
                                child.estimate_value=estimate;child.priority=child.cost+estimate;
                                next.push_back(child);
                            }else if(child.cost<next[it->second].cost) {
                                child.estimate_value=next[it->second].estimate_value;
                                child.priority=child.cost+child.estimate_value;
                                next[it->second]=child;
                            }''')
    change('                if(expanded==192){++stats.capped;stop=true;break;}\n','''#ifdef V116_COLLECTOR
                if(expanded==finite_probe::node_limit){++stats.capped;stop=true;break;}
#else
                if(expanded==192){++stats.capped;stop=true;break;}
#endif
''')
    change('            int width=min(12,int(next.size()));stats.beam_pruned+=int(next.size())-width;\n','''#ifdef V116_COLLECTOR
            int width=min(finite_probe::width,int(next.size()));
#else
            int width=min(12,int(next.size()));
#endif
            stats.beam_pruned+=int(next.size())-width;
''')
    change('                            ++stats.generated;\n','''                            ++stats.generated;
#ifdef V116_COLLECTOR
                            if(finite_probe::edge && !finite_probe::edge(from.state,child.state,path)) {
                                stop=true;break;
                            }
#endif
''')
    change('        trace.count_by("floor_cells", board_info.cell_count);','''        trace.count_by("finite_value_calls",finite_value::calls);
        trace.count_by("floor_cells", board_info.cell_count);''')
    # Reverse exactly the recorded additions: all other solver code must match v111.
    restored=text
    for before,after in reversed(edits):
        assert restored.count(after)==1
        restored=restored.replace(after,before,1)
    assert restored==original
    output=Path(output).absolute();text='// '+output.name+'\n'+text.split('\n',1)[1]
    assert len(text.encode())<=512000
    output.parent.mkdir(parents=True,exist_ok=True);output.write_text(text)
    return dict(source=str(output.relative_to(ROOT)),sha256=sha(output),bytes=len(text.encode()),
                parent_sha256=PARENT_SHA,model_sha256=sha(Path(model)) if model else None,
                inverse_patch_equals_parent=True)


def checker(output,source):
    text=(ROOT/'adhoc/bin/collect_v116_finite.cpp').read_text()
    text=text.replace('#include "v116_finite_value.cpp"','#include "'+Path(source).name+'"')
    start=text.index('void compare(')
    text=text[:start]+(ROOT/'adhoc/bin/v121_check_tail.hpp').read_text()
    text='// '+Path(output).name+'\n'+text.split('\n',1)[1]
    Path(output).write_text(text)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--model',type=Path)
    p.add_argument('--checker',type=Path)
    a=p.parse_args();print(json.dumps(build(a.output,a.model),indent=2))
    if a.checker:checker(a.checker,a.output)
