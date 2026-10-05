#!/usr/bin/env python3
"""Record completed experiments only; never executes or changes a solver.
Reads the current backlog and changes only B-133 plus one new v315 entry.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
path = ROOT / 'notes/backlog.md'
text = path.read_text()
marker = '## 決着済み\n'
assert text.count(marker) == 1
for version in ('v314', 'v315'):
    assert (ROOT / f'notes/experiments/{version}.md').is_file()

support = ('- **[B-133] 支持を失うジャンプと支持個体の再挿入を同時に決める** → '
           '[v314](experiments/v314.md) この固定比較では候補。元の1ジャンプ案を、1匹に対する複数の必須支持時刻へ具体化した。'
           '仮の予定をそのまま出力せず、最後の需要より早い帰巣を除き、支持条件を満たした完成経路だけを通常修復と比較する。'
           '789部分問題で通常経路は親と一致し、通常615件に対し条件付き785件が完成、170件を新規に完了した。既存経路短縮後の共通成功例でも40件を短縮した。'
           'GCC14.2・非LOCAL・2並列のin100は親v311標準比81手減、固定参照相対+0.227065ポイント、補助100件は41手減。'
           'ただしin0005だけで75手減り、時間による初期生成打ち切りで保持候補が親1本から3本へ変わった。除く99件は6手減で、全差を条件付きDPだけの効果としない。'
           '新旧3版の全600出力が合法・全帰巣・外部1.9秒以内。195手は未達、各入力1回で再実行ノイズ未測定。'
           '再開条件: 同じ候補のユーザー環境での比較、または初期生成と探索条件を固定した別仮説への明示指示。')
pattern = r'^- \*\*\[B-133\][^\n]*(?:\n(?!\n|## |- \*\*)[^\n]+)*'
match = re.search(pattern, text, flags=re.M)
assert match, 'B-133 missing; stop rather than overwrite unrelated backlog'
if '[v314](experiments/v314.md)' not in match.group():
    assert text.index('## 未着手') < match.start() < text.index(marker), 'B-133 was changed concurrently'
    text = text[:match.start()] + text[match.end():]
    text = text.replace(marker, marker + '\n' + support + '\n', 1)

if '[v315](experiments/v315.md)' not in text:
    number = max(map(int, re.findall(r'\[B-(\d+)\]', text))) + 1
    racing = (f'- **[B-{number}] 接戦の初期計画を途中まで二候補で育成する** → '
              '[v315](experiments/v315.md) この固定比較では候補。v110の同じ探索経過を使う診断から、v311の30%時点の永久な一択を見直す。'
              'NNとLNS本体を保持し、接戦の次点へ定期的に時間を配り、80%以後は最良候補へ集中する。'
              '同じGCC14.2・非LOCAL・2並列のin100はv311標準比108手減、固定参照相対+0.115260ポイント、補助100件は124手減。'
              'in0005の81手減には初期候補数の違いがあり、除く99件は27手減だった。初期生成のコード不変から候補列や時計の一致を保証しない。'
              'inの挑戦側への配分は94区間、30%時点からの最終選択変更は7入力。全600出力が合法・全帰巣・外部1.9秒以内。'
              '195手未達、単回比較で効果量は未確定。再開条件: 同環境の追加比較または別の配分仮説への明示指示。')
    text = text.replace(marker, marker + '\n' + racing + '\n', 1)
    print('v315 backlog ID:', number)
path.write_text(text)
print('Only B-133 and the v315 completed entry were changed.')
