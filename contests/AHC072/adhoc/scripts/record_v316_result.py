#!/usr/bin/env python3
"""Record completed isolated results only. Does not execute any solver."""
from pathlib import Path
import hashlib,json,re
ROOT=Path(__file__).resolve().parents[2]
S=json.loads((ROOT/'adhoc/v316/evaluation_summary.json').read_text())
assert S['complete'] and S['evaluated_outputs']==800
assert hashlib.sha256((ROOT/'src/bin/v316_integrated_neural_lns.cpp').read_bytes()).hexdigest()==S['source_sha256']
text=(ROOT/'adhoc/v316/result_note.md').read_text().rstrip()
note=ROOT/'notes/experiments/v316.md'
old=note.read_text()
marker='## 実験後\n\n未実行。'
if marker in old:
    assert old.count(marker)==1
    note.write_text(old.replace(marker,'## 実験後\n\n'+text))
else:
    assert text in old,'Another result already exists; do not overwrite it'
backlog=ROOT/'notes/backlog.md'
b=backlog.read_text()
title='条件付き支持と接戦育成へ差分推論の時間を回す'
if title not in b:
    numbers=[int(n) for n in re.findall(r'\[B-(\d+)\]',b)]
    number=max(numbers+[0])+1
    i=S['comparisons']['in'];a=S['comparisons']['validation100']
    c1=i['v316_vs_v314'];c2=i['v316_vs_v315'];q1=a['v316_vs_v314'];q2=a['v316_vs_v315']
    entry=(f'- **[B-{number}] {title}** → [v316](experiments/v316.md) 不採用（今回の固定条件）。'
           f'v314の条件付き支持、v315の二候補育成、v401のCNN差分更新を統合し、全量CNNの統合のみも事前固定した。'
           f'GCC14.2・非LOCAL・2並列のin100で標準はv314比{c1["delta_total"]:+d}手、固定相対{c1["delta_relative_pp"]:+.6f}ポイント、'
           f'v315比{c2["delta_total"]:+d}手、同{c2["delta_relative_pp"]:+.6f}ポイント。'
           f'補助100件はv314比{q1["delta_total"]:+d}手、v315比{q2["delta_total"]:+d}手。'
           '保存19,904局面と8,616,814 logitsは全量参照とbit一致し、固定列のencodeは約2.25倍になったが、inの両親超えは未達。'
           '4条件の全800出力が合法かつ全帰巣。195手未達。各入力1回で因果効果や再実行ノイズは未確定。'
           '再開条件: 追加比較や新しい配分の明示指示。eval viewerと共有評価ログは未変更。\n\n')
    heading='## 決着済み\n\n'
    assert b.count(heading)==1
    backlog.write_text(b.replace(heading,heading+entry,1))
    print('Registered B-'+str(number))
print('v316 completed record preserved')
