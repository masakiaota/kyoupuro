#!/usr/bin/env python3
"""Record completed evidence only. Never run solvers, train or register evals."""
from pathlib import Path
import base64,csv,hashlib,io,json,math,struct,zlib
ROOT=Path(__file__).resolve().parents[2]
SOURCE='931b5b40e4595c9a500f1d5a2f978e067fc867a26137d8b9a55f4d13e5ee8cbb'
RAW_SHA='d9417844e6b6c50bbdd8c180590adde3e4f217963fdf4e70a3eb2826aa77893e'
assert hashlib.sha256((ROOT/'src/bin/v319_integrated_learned_racing.cpp').read_bytes()).hexdigest()==SOURCE
raw=zlib.decompress(base64.b64decode((ROOT/'adhoc/v319/paired_payload.b64').read_text()))
assert len(raw)==2600 and hashlib.sha256(raw).hexdigest()==RAW_SHA
versions=('v315','v111','v319');records=[]
for i in range(600):
 a,b,c=struct.unpack_from('<Hbb',raw,4*i)
 group,case=('in',i) if i<100 else ('validation1',i-100)
 ref=struct.unpack_from('<H',raw,2400+2*i)[0] if i<100 else ''
 records.append(dict(set=group,case=f'{case:04d}.txt',v315=a,v111=a+b,v319=a+c,v800_reference=ref))
expected={'in':[19867,19833,19763],'validation1':[108650,108271,108275]}
maxima={'in':[1.8854785820000188,1.8858788499999264,1.8851138490000494],
        'validation1':[1.889576820000002,1.8860495609999361,1.9020251389999885]}
summary=dict(experiment='v319',source_sha256=SOURCE,main_snapshot='0cceb2ed9cb377582f378455e7a9fe17022e9d8e',
 conditions=dict(compiler='GCC14.2',standard='C++23',LOCAL=False,workers=2,budget_seconds=1.9,internal_deadline_seconds=1.88,repetitions=1,reference='100*mean(original v800 S / candidate S), no clipping',validators='Independent Python and C++ literal color stacks; official Rust binary NOT executed'),
 accept_candidate=False,target195_met=False,all1800_valid_complete=True,diagnostic_errors=0,sets={},comparisons={})
for group in ('in','validation1'):
 rows=[r for r in records if r['set']==group];summary['sets'][group]={};summary['comparisons'][group]={}
 for j,v in enumerate(versions):
  total=sum(r[v] for r in rows);assert total==expected[group][j]
  x=dict(cases=len(rows),sum_S=total,mean_S=total/len(rows),complete=len(rows),E_total=0,max_external_seconds=maxima[group][j],over_1_9=int(group=='validation1' and v=='v319'),over_2=0)
  if group=='in':x['relative_v800']=math.fsum(100*r['v800_reference']/r[v] for r in rows)/len(rows)
  summary['sets'][group][v]=x
 for old in ('v315','v111'):
  delta=[r['v319']-r[old] for r in rows]
  x=dict(delta_sum=sum(delta),delta_mean=sum(delta)/len(rows),wins=sum(d<0 for d in delta),ties=sum(d==0 for d in delta),losses=sum(d>0 for d in delta))
  if group=='in':x['relative_v800_delta']=summary['sets'][group]['v319']['relative_v800']-summary['sets'][group][old]['relative_v800']
  summary['comparisons'][group]['v319_vs_'+old]=x
assert sum(r['v800_reference'] for r in records[:100])==18439
buf=io.StringIO();writer=csv.DictWriter(buf,fieldnames=list(records[0]),lineterminator='\n');writer.writeheader();writer.writerows(records)
def write_same(path,text):
 p=ROOT/path;p.parent.mkdir(parents=True,exist_ok=True)
 if p.exists() and p.read_text()!=text:raise RuntimeError('Different existing evidence: '+path)
 p.write_text(text)
write_same('adhoc/v319/paired_scores.csv',buf.getvalue())
write_same('adhoc/v319/evaluation_summary.json',json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
post='''
## 実験後

### 判定

採用条件未達。現行v315より両集合で短縮したが、副対照v111に対するvalidation1全500件の非悪化条件を満たさなかった。現行指定は自動変更しない。in平均195手以下も未達である。絶対値は[評価要約](../../adhoc/v319/evaluation_summary.json)、各入力は[比較CSV](../../adhoc/v319/paired_scores.csv)を参照する。

- in100はv315比104手減、元v800固定参照相対+0.528624ポイント、53勝18分29敗。v111比70手減、相対+0.291385ポイント、46勝20分34敗。
- validation1全500件はv315比375手減、244勝48分208敗。v111比4手増（平均0.008手）、204勝89分207敗。小差であり、安定した劣化とは断定しない。
- 0005を除いたin99件でもv315比128手減、v111比58手減だった。inの平均差の入力再標本化95%区間は対v315が約[-2.230,+0.180]、対v111が[-1.490,+0.100]手。validationではそれぞれ約[-1.352,-0.168]、[-0.394,+0.412]手。これらは入力構成の区間で、再実行ノイズを測ったものではない。

### 機構と完成版

事前のNN出力と履歴集計の同値性検査を通過した。v111とv319の原方向NN列hashは全600入力で一致した。保持候補の由来と長さはin99件、validation497件で一致したが、全追加操作列や時計の一致を意味しない。

inのv111比ではLNS開始が平均5.665ms早まり、候補管理時間は12.576ms減、LNS試行数は約2.09%増えた。validationでは開始6.703ms短縮、管理13.242ms減、試行約1.01%増だった。inの最短初期解の合計はv111と一致している。個別のcacheや育成配分の独立効果は分離せず、組み合わせ全体の結果とする。

### 条件と保存

GCC14.2、C++23、LOCALなし、親と同じ1.9秒予算、2並列、各入力各条件1回。in100とvalidation1全500件をすべて測定し、同じ入力内の3版を同じCPUで連続実行、6順序を循環した。既存検証集合を新規未見とは呼ばない。

全1,800出力が独立Python/C++で合法かつ全帰巣、T/E/S一致、診断エラー0だった。公式Rustバイナリは実行していない。外部時間最大はv319の1.902025秒。v319の1件だけ1.9秒を超え、他の1,799件は以内、全件2秒以内だった。元アップロードのv800単独100値を固定参照にし、対応する100入力のSHA256一致を確認した。歴代MINは別名の参考値に限る。

完全CPP、実験ノート、比較結果、台帳をmainへ直接保存した。CPPのSHA256は931b5b40e4595c9a500f1d5a2f978e067fc867a26137d8b9a55f4d13e5ee8cbb。全出力、stderr、入力、完全前処理、機構診断、凍結manifest、詳細集計は会話添付AHC072_v319_evaluation_bundle.zipへ保存する。事前登録はローカルで実装前に保存し、Gitへの反映は本評価開始後だった。この時刻差を区別する。

次の前提: 学習後方策、接戦育成と計算再利用の統合は現行v315に対して両集合で改善を観測したが、v111に対してはinとvalidationで方向が分かれた。普遍的な最強性や各部品の加算可能な効果を確定しない。再開条件は新しい明示指示によるユーザー環境の同一ソース比較、または別の具体的統合仮説。結果からのsolver修正、追加学習、再評価、別ブランチ、force push、AtCoder投稿、eval viewerと共有評価ログの更新は行っていない。
'''
p=ROOT/'notes/experiments/v319.md';text=p.read_text()
if '\n## 実験後\n' not in text:p.write_text(text+post)
elif post not in text:raise RuntimeError('Different existing result note')
ledger='- **[B-170] 学習後方策と接戦育成へ厳密な計算再利用を統合する** → [v319](experiments/v319.md) 採用条件未達。v111固定重み、v315育成、v405のCNN/actor再利用、v318履歴を統合。GCC14.2、非LOCAL1.9秒予算、2並列でin100とvalidation1全500件を対照2版と各1回比較。inはv315比104手減、元v800固定相対+0.528624ポイント、v111比70手減。validationはv315比375手減、v111比4手増で副対照の非悪化条件未達。全1800出力合法かつ全帰巣、最大1.902025秒、2秒超0件。同値性を確認したが、各部品の寄与と再実行ノイズは未分離。現行指定を自動変更しない。再開は新しい明示指示による同一ソース比較か別の統合仮説。'
p=ROOT/'notes/backlog.md';text=p.read_text();old=[x for x in text.splitlines() if x.startswith('- **[B-170]')]
assert len(old)==1 and 'v319' in old[0]
if old[0]!=ledger:
 text=text.replace(old[0]+'\n','',1);assert '## 決着済み\n' in text
 p.write_text(text.replace('## 決着済み\n','## 決着済み\n\n'+ledger+'\n',1))
print('Recorded v319: 600 paired inputs, completed note and ledger; no solver runs')
