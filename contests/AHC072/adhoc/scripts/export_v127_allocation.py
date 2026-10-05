#!/usr/bin/env python3
"""予測費用を最初の選抜だけに使い、その後のv113の育成を保持する。"""
from build_v127_allocation import ROOT, RUN, BASE, BASE_SHA, grow_tracking
from export_v124_allocation import build_candidate as build_full_candidate
from check_v113_integrated import block
from v089_data import sha


def build_candidate(model_path, name):
    assert sha(BASE) == BASE_SHA
    target = build_full_candidate(model_path, name)
    text = target.read_text()
    parent = BASE.read_text()
    rank = block(text, 'auto allocation_rank=[&]')
    grow = grow_tracking(parent)
    grow = grow.replace('    try {\n        // 最初の2段階',
                        '    ' + rank + ';\n    try {\n        // 最初の2段階', 1)
    old = '''            sort(active.begin(),active.end(),[&](int a,int b) {
                const size_t left=pool.entries[a].moves.size(),right=pool.entries[b].moves.size();
                return left!=right?left<right:a<b;
            });'''
    assert grow.count(old) == 1
    grow = grow.replace(old, '            if(round==0) allocation_rank(active);\n            else {\n' + old + '\n            }', 1)
    needle = '            active.resize(round==0?(active.size()+1)/2:min<size_t>(2,active.size()));'
    assert grow.count(needle) == 1
    # 選抜後の育成順は親へ戻し、予測順位による時間配分の追加変更を避ける。
    grow = grow.replace(needle, needle + '\n            if(round==0) {\n' + old + '\n            }', 1)
    text = text.replace(block(text, 'void grow_initial_solutions('), grow, 1)
    text = text.replace(block(text, 'struct ReactiveRaceStats {'), block(parent, 'struct ReactiveRaceStats {'), 1)
    target.write_text(text)
    return target
