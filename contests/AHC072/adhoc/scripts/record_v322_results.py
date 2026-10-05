#!/usr/bin/env python3
"""Record completed evidence only. No solver, training, or eval registration."""
from pathlib import Path
import csv, hashlib, io, json, statistics
ROOT=Path(__file__).resolve().parents[2]
SOURCE_SHA='abb46ec1d213e38840fd3780d9fdfc512f17383faa9b5234070d809bb9789fd9'
DATA_SHA='76c9028c5b4f6c9d84883d1e0d4caec28a9f125475d56d9d6a7fb86ce2e9aca2'
DATA='''495,497,0
120,122,0
325,341,0
140,140,0
134,132,0
151,145,0
361,374,0
207,202,0
308,314,0
244,241,0
70,69,0
219,211,0
186,194,0
210,211,0
64,65,0
107,105,0
157,160,0
127,129,0
373,364,0
287,288,0
596,591,0
138,132,0
247,246,0
93,96,0
82,82,0
526,521,0
224,219,0
207,205,0
324,308,0
127,125,0
380,367,0
151,147,0
119,119,0
110,109,0
116,111,0
115,115,0
161,163,0
77,78,0
202,205,0
84,87,0
464,458,0
362,366,0
112,106,0
89,90,0
132,128,0
375,371,0
290,302,0
101,102,0
574,572,0
134,137,0
433,432,0
238,239,0
296,293,0
152,158,0
344,346,0
425,419,0
115,112,0
167,169,0
166,169,0
390,389,0
161,167,0
118,115,0
350,343,0
411,406,0
415,420,0
309,297,0
635,628,0
270,276,0
110,108,0
124,119,0
125,126,0
117,113,0
70,67,0
150,158,0
218,221,0
282,271,0
358,358,0
145,145,0
209,212,0
334,324,0
426,401,0
285,284,0
114,114,0
79,79,0
91,87,0
340,346,0
208,208,0
261,262,0
165,174,0
409,419,0
317,310,0
55,55,0
127,131,0
622,633,0
231,236,0
217,222,0
535,529,0
69,72,0
119,124,0
389,370,0
293,295,0
476,477,0
395,372,0
186,180,0
155,157,0
147,146,0
220,229,0
418,417,0
217,224,0
128,129,0
280,282,0
390,390,0
224,226,0
137,129,0
223,210,0
228,239,0
83,84,0
117,119,0
201,205,0
313,310,0
310,310,0
504,499,0
190,193,0
196,196,0
105,106,0
204,208,0
191,202,0
273,267,0
43,43,43
101,101,100
108,108,104
120,123,115
242,249,234
626,613,483
117,117,107
111,112,110
125,122,120
83,83,80
69,70,69
150,154,145
122,123,119
180,180,168
352,342,331
510,508,459
123,124,119
191,193,186
72,72,70
591,587,512
139,142,133
65,66,64
69,69,65
84,84,76
119,121,115
57,56,56
125,125,118
139,138,130
240,251,234
241,240,227
422,417,382
120,120,117
149,140,138
136,137,127
400,408,373
63,62,62
203,202,183
246,246,228
128,131,120
116,118,107
137,139,130
84,90,84
296,303,292
165,159,148
181,183,163
192,187,184
149,147,139
308,297,288
225,223,205
187,187,175
136,142,135
118,120,113
187,192,174
127,127,121
287,285,264
276,282,254
194,191,185
230,227,218
312,300,269
68,67,66
151,155,141
266,267,256
166,166,158
261,263,230
185,188,182
131,126,114
393,375,359
197,200,188
396,393,354
130,134,124
388,397,362
391,393,357
91,91,87
229,226,208
95,96,92
342,339,315
259,274,234
82,82,81
271,273,236
67,68,65
177,169,159
346,345,309
310,310,282
170,168,156
315,306,275
210,194,187
94,95,92
112,109,107
143,148,143
335,348,314
150,156,147
99,100,96
513,508,451
82,84,81
116,115,114
243,230,214
85,86,78
546,542,464
123,127,120
177,181,171
'''
assert hashlib.sha256(DATA.encode()).hexdigest()==DATA_SHA
assert hashlib.sha256((ROOT/'src/bin/v322_sparse_plan_rewriting.cpp').read_bytes()).hexdigest()==SOURCE_SHA
pairs=[tuple(map(int,line.split(','))) for line in DATA.splitlines()]
assert len(pairs)==228
out=ROOT/'adhoc/v322';out.mkdir(parents=True,exist_ok=True)
buf=io.StringIO();writer=csv.writer(buf,lineterminator='\n');writer.writerow(['set','case','v113','v322','v800_reference'])
summary={'verdict':'not_adopted','reason':'in relative score decreased','source_sha256':SOURCE_SHA,'compiler':'GCC14.2','LOCAL':False,'nominal_budget_seconds':1.9,'jobs':2,'replicates':1,'validation1_used':False,'sets':{}}
for name,start,n in [('fresh128',0,128),('in',128,100)]:
    group=pairs[start:start+n];diff=[b-a for a,b,r in group]
    stats={'n':n,'delta_sum':sum(diff),'delta_mean':statistics.mean(diff),'wins':sum(x<0 for x in diff),'draws':sum(x==0 for x in diff),'losses':sum(x>0 for x in diff)}
    for col,v in enumerate(('v113','v322')):
        stats[v]={'sum':sum(row[col] for row in group),'mean':statistics.mean(row[col] for row in group)}
        if name=='in':stats[v]['relative_v800_mean_100']=100*statistics.mean(row[2]/row[col] for row in group)
    summary['sets'][name]=stats
    for i,(a,b,r) in enumerate(group):writer.writerow([name,f'{i:04d}.txt',a,b,r or ''])
assert summary['sets']['fresh128']['delta_sum']==-78 and summary['sets']['in']['delta_sum']==-21
summary['legality']={'outputs':456,'legal_complete':456,'official':'Rust vis --no-vis','independent':'Python lists','diagnostic_errors':0,'warmups_excluded':2}
summary['time']={'v113_max':1.92597519,'v322_max':1.940659383,'v113_over_1_9':1,'v322_over_1_9':3,'over_2':0}
summary['mechanism']={'atomic_conditions':393216,'fixture_outputs_official_checked':8,'rewrite_attempts':33527,'rewrite_completed':84,'rewrite_raw_saved':94,'long_span_completions':9,'max_completed_span':246,'new_first_nn_hash_matches':228}
summary['data_payload_sha256']=DATA_SHA
(out/'paired_scores.csv').write_text(buf.getvalue())
(out/'evaluation_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
POST='''

## 実験後

### 判定

**不採用。現行v113を維持する。** 新規128件は親比78手減（平均0.609375手）、in100は21手減（平均0.21手）だった。しかし、inのv800単独固定参照相対値は0.153075ポイント下がり、事前の採用条件を満たさない。in平均195手以下と厳密な全件外部1.9秒以内も未達である。絶対値は[要約](../../adhoc/v322/evaluation_summary.json)、全入力は[比較CSV](../../adhoc/v322/paired_scores.csv)を参照する。

新規は56勝13分59敗、inは37勝18分45敗だった。0005を除いたin99件は8手減。入力bootstrap95%区間は新規の平均差が約[-1.7344,+0.5002]手、inが[-1.29,+0.83]手で、再実行ノイズを含まない。安定した優位や劣化とは断定しない。

### 機構と完成版

393,216の人工遷移条件で、差分表現と独立した可変長塔の全盤面再生が一致した。4本の手組み列は8→6、88→86、12→10、9→8手となり、元列と書換え後の計8出力を公式Rustと独立Pythonで確認した。80操作の無関係な列をまたぐ接続も成立した。

実入力では33,527試行中84件の書換えが完成し、直接の短縮合計は94手だった。新規49件、in35件。64操作を超える接続は9件、最長246操作である。新規40入力とin27入力で完成書換えがあった。平均費用は新規14.061ms、in15.885ms。同一探索内の直接短縮や最良更新幅は、親に対する独立な改善量とはしない。

NN原方向の操作列hashは全228入力で親と一致した。最短初期解合計は両集合とも親と一致し、保持候補の由来と長さは新規125/128、in96/100で一致した。追加列の全操作や実時計が同一という意味ではない。LNS試行数は新規約3.80%、in約0.68%増えたが、不成立や新近傍の試行も含むため純粋な速度倍率とはみなさない。

LOCALと非LOCALの両ビルド、完全前処理でNN、時計、通常再挿入、束再挿入、二順序、順位NN、育成の保持を確認した。最初のprobeビルドのtarget属性不一致は診断コードとビルド条件だけを修正した。提出solverと評価条件は変更していない。

### 条件と限界

GCC14.2/C++23、非LOCAL、親の1.9秒基準、2並列、各入力各版1回。新規は公式seed3220000000000から64刻みの128件で、inと内容重複なし。validation1は実行、学習、重複検査のいずれにも使っていない。過去の全データとの完全非重複を保証したものではない。

本評価456出力が公式Rust visと独立Pythonで合法かつ全帰巣し、手数と公式スコアが一致した。診断エラー0。外部最大は親1.925975秒、新版1.940659秒、1.9秒超は親1件、新版3件で、全件2秒以内だった。ウォームアップ2本は集計に含めない。時間超過ケースも削除や再実行をしていない。

### 調査と保存

14本の一次資料を比較し、計画の局所書換え、MAPF-LNS、SIPP、SISR、適応近傍、学習による部分問題選択、方策誘導探索などを[調査記録](../../adhoc/v322/research_review.md)へ整理した。多くは抄録までの確認である。Planning by Rewritingの本文3.1.3〜3.1.5を参照したが、その規則言語の再現ではなくゲーム向けの独自近傍である。他問題の改善率をAHC072へ外挿しない。

完全CPPのSHA256はabb46ec1d213e38840fd3780d9fdfc512f17383faa9b5234070d809bb9789fd9。Gitのblob SHAも評価済みファイルと一致した。全出力、stderr、入力、独立採点、凍結manifest、完全前処理、診断、詳細報告は会話添付AHC072_v322_evaluation_bundle.zipに保存する。転送の文字列修復はhash照合付きで行い、solverは不変。Actionsはソース取得と完了済み資料の記録だけに使った。

次の前提: 長い期間の操作変更を少数マスの色盤面差で扱える一方、今回の一様な開始操作抽選と固定探索幅では完成率が約0.251%に留まった。大幅改善は未達であり、今回の結果を見た条件調整、追加学習、solver再評価は行わない。再開には新しい具体的な仮説と明示指示が必要である。mainへ直接保存し、現行指定、eval viewer、共有評価ログ、AtCoder投稿は変更しない。
'''
note=ROOT/'notes/experiments/v322.md';text=note.read_text()
if '## 実験後' not in text:note.write_text(text+POST)
ledger=ROOT/'notes/backlog.md';text=ledger.read_text()
entry='\n- **[v322] 少数マスの差分による操作列の局所書換え** → [v322](experiments/v322.md) 不採用。v113を親に1操作の省略で生じる色盤面差を最大4マスで追い、後続の分割と出発点を直す。33,527試行中84件を完成し、最長246操作の接続が成立した。新規128件は78手減、in100は21手減だがv800固定参照相対は0.153075ポイント低下。全456出力が公式採点と独立再生で合法かつ全帰巣、最大1.940659秒。validation1未使用。14一次資料の調査と実装、凍結比較を保存した。現行v113を維持し、結果を見た改変は行わない。\n'
if '**[v322] 少数マスの差分による操作列の局所書換え**' not in text:
    if '## 決着済み' in text:text=text.replace('## 決着済み','## 決着済み\n'+entry,1)
    else:text+='\n## v322の完了記録\n'+entry
    ledger.write_text(text)
print('Verified and recorded completed v322 evidence; no solver execution')
