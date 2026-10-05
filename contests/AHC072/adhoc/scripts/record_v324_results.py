#!/usr/bin/env python3
"""Record completed evidence only; no solver, training, or eval registration."""
from pathlib import Path
import csv,hashlib,io,json,statistics
root=Path(__file__).resolve().parents[2];d=root/'adhoc/v324'
p=json.loads((d/'score_payload.json').read_text());m=json.loads((d/'metadata.json').read_text())
source=root/'src/bin/v324_repair_cost_tournament.cpp'
assert hashlib.sha256(source.read_bytes()).hexdigest()==m['source_sha256']
assert len(p['fresh128'])==128 and len(p['in'])==100 and len(p['v800_reference'])==100
out=io.StringIO();w=csv.writer(out,lineterminator='\n');w.writerow(['set','case','v113','v324','v800_reference','delta'])
m['sets']={}
for name in ('fresh128','in'):
    pairs=p[name];n=len(pairs);differences=[b-a for a,b in pairs]
    e={'n':n,'delta_sum':sum(differences),'delta_mean':statistics.mean(differences),'wins':sum(x<0 for x in differences),'ties':sum(x==0 for x in differences),'losses':sum(x>0 for x in differences)}
    for j,v in enumerate(('v113','v324')):
        scores=[pair[j] for pair in pairs];e[v]={'sum':sum(scores),'mean':statistics.mean(scores)}
        if name=='in':e[v]['relative_v800']=statistics.mean(100*r/t for r,t in zip(p['v800_reference'],scores))
    if name=='in':e['relative_delta']=e['v324']['relative_v800']-e['v113']['relative_v800']
    m['sets'][name]=e
    for i,(a,b) in enumerate(pairs):w.writerow([name,f'{i:04d}.txt',a,b,p['v800_reference'][i] if name=='in' else '',b-a])
assert hashlib.sha256(out.getvalue().encode()).hexdigest()==p['csv_sha256'],'transferred data mismatch'
assert m['sets']['in']['delta_sum']==12 and m['sets']['fresh128']['delta_sum']==66
(d/'paired_scores.csv').write_text(out.getvalue());(d/'evaluation_summary.json').write_text(json.dumps(m,indent=2)+'\n')
note=root/'notes/experiments/v324.md';post=(d/'completed_note.md').read_text();old=note.read_text()
if post.strip() not in old:
    assert '## 実験後' not in old,'new results appeared; merge explicitly'
    note.write_text(old.rstrip()+'\n'+post)
ledger=root/'notes/backlog.md';text=ledger.read_text()
item='\n- **[v324] 修復後の実手数で除去候補を選ぶ** → [v324](experiments/v324.md) 不採用。v113の通常優先選択で、修復費が2手以上の場合に同じ匹数の候補を最大2個追加検査し、短い背景列へ切り替える。保存247部分問題の再挿入完成は75→109件だが、完成版in100は12手増、v800固定参照相対−0.066135ポイント、新規128件も66手増。追加検査は平均22〜25ms、inのLNS試行数は約4.68%減。全456出力が公式採点と独立再生で合法かつ全帰巣、最大1.886480秒。validation1とeval viewerは未使用、現行v113を維持。単回比較のため恒常的な劣化は未確定。再開条件: 新しい具体的仮説と明示指示。\n'
if '**[v324]' not in text:ledger.write_text(text.rstrip()+'\n'+item)
print('Recorded 228 paired inputs; no solver execution or evaluation registration.')
