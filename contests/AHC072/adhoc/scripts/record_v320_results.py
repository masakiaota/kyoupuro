#!/usr/bin/env python3
"""Record completed v320 evidence only; never run solvers or training."""
from pathlib import Path
import base64,csv,hashlib,io,json,zlib
ROOT=Path(__file__).resolve().parents[2]
EXPECTED_SOURCE='9230c3d0ee6ec5ad955f862b81eec70ce682b3f4087983631f67948ab60d23f8'
EXPECTED_CSV='d4a8bdcb8f489b237f0589c465d188c6ebdce743a027a6ec49427412e95bf490'
def sha(b):return hashlib.sha256(b).hexdigest()
assert sha((ROOT/'src/bin/v320_instance_router.cpp').read_bytes())==EXPECTED_SOURCE
p=ROOT/'adhoc/v320/paired_scores.b64'
encoded=p.read_text().strip()
# Repair a known accidental insertion in transport, not any measurement.
# The independent full CSV checksum is mandatory after decoding.
encoded=encoded.replace('JAjhkx6aZow','JAjhk6aZow')
data=zlib.decompress(base64.b64decode(encoded,validate=True))
assert sha(data)==EXPECTED_CSV,'Refusing to publish changed measurements'
rows=list(csv.DictReader(io.StringIO(data.decode())))
assert len(rows)==548 and {r['set'] for r in rows}=={'train','holdout','in'}
sets={}
for split,count in [('train',320),('holdout',128),('in',100)]:
    selected=[r for r in rows if r['set']==split];assert len(selected)==count
    versions={}
    for v in ['v111','v113','v315','v210','v320']:
        if not selected[0].get(v):continue
        scores=[int(r[v]) for r in selected];baseline=[int(r['v111']) for r in selected]
        versions[v]={'cases':count,'sum':sum(scores),'mean':sum(scores)/count,
            'delta_vs_v111':sum(scores)-sum(baseline),
            'wins_vs_v111':sum(a<b for a,b in zip(scores,baseline)),
            'ties_vs_v111':sum(a==b for a,b in zip(scores,baseline)),
            'losses_vs_v111':sum(a>b for a,b in zip(scores,baseline))}
        if split=='in':versions[v]['v800_relative']=sum(100*int(r['v800_reference'])/int(r[v]) for r in selected)/count
    sets[split]=versions
assert sets['in']['v320']['sum']==20052 and sets['holdout']['v320']['sum']==28586
summary={'version':'v320','verdict':'rejected','sets':sets,'source_sha256':EXPECTED_SOURCE,
 'csv_sha256':EXPECTED_CSV,'source_snapshot':'85cbc6a8c5919810612c51097ca306bede7e32b2',
 'compiler':'GCC14.2 C++23','local_macro':False,'parent_nominal_budget_seconds':1.9,'parallel_jobs':2,
 'official_and_independent_valid_outputs':2320,'validation1_solver_runs':0,'validation1_training_rows':0,
 'v320_final_cases':228,'v320_external_max_seconds':1.9312339969997083,'v320_external_over_1_9':223,
 'v320_external_over_2':0,'all_experts_external_max_seconds':2.0475952990000224,'all_experts_external_over_2':3,
 'routing_features':17,'selected_depth':1,'threshold':0.10574372577025097,
 'dispatch':{'train':{'v315':80,'v113':240},'holdout':{'v315':27,'v113':101},'in':{'v315':32,'v113':68}},
 'routing_startup_mean_us':{'holdout':250.38777343750004,'in':241.86148},
 'cpp_python_feature_max_error':0,'final_choice_mismatches':0,'source_changed_after_freeze':False,
 'fixed_expert_selector_replay':{'holdout':28551,'in':20027},
 'uncertainty':'One timed measurement per input and version, excluding warmup. Runtime noise was not remeasured.'}
(ROOT/'adhoc/v320/paired_scores.csv').write_bytes(data)
p.write_text(encoded+'\n')
(ROOT/'adhoc/v320/evaluation_summary.json').write_text(json.dumps(summary,indent=2,ensure_ascii=False)+'\n')
POST='''
## 実験後

### 判定

不採用。現行v111を維持する。in100はv111比15手増、v800固定参照相対−0.153418ポイント、34勝17分49敗だった。新規保留128件は58手減、61勝20分47敗だったが、両集合の絶対非悪化とin相対改善という事前条件に届かなかった。平均195手も未達である。固定v113と比べても、inで57手、新規保留で52手多かった。絶対値と全ケースは[独立比較CSV](../../adhoc/v320/paired_scores.csv)、[評価要約](../../adhoc/v320/evaluation_summary.json)へ保存する。

### 新規入力から学んだ条件

公式genの新規320入力で固定4版を1回ずつ測り、全1,280出力を合法かつ全帰巣と確認した。最大深さ0/1/2、最小葉40、固定の5分割交差検証で深さ1を選んだ。同色の上下左右の隣接個体を持つスライムの割合が0.10574372577025097以下ならv315、それ以外はv113を選ぶ。学習集合の割当は80/240件である。交差検証の固定v113への差は平均0.06875手の短縮にとどまった。

木とソースとビルドを固定してから、新規保留128件とin100を開いた。保留は27/101件、inは32/68件へ割り当てた。新規保留とinの出力から条件を調整せず、別の専門家へ差し替えていない。固定v113をinの対照へ追加することはin開始前かつ保留スコア閲覧前に記録し、採否基準は保持した。

元版の測定済み出力から条件で選ぶ参考計算でも、v113固定に対して新規保留17手、in32手多かった。実際のCPPはその参考計算より35手/25手多く、時間依存探索と組み込みの実行差を含む。この差だけを特徴計算の費用に帰属しない。新規保留の4版の事後最良には余地があるが、4回の計算を使う参考値であり、1.9秒での達成性能ではない。

### 機構、合法性、時間

最終CPPは選択されたv113とv315だけを名前空間へ格納し、初期化と入力処理を含む共通時計で1版だけを実行する。重みは不変。ソース共有後は308,024バイトで、共有前後の完全前処理のトークンはLOCAL/非LOCALで一致した。固定時計の16組で元版と組込先の出力一致を確認した。学習320件の5,440特徴値はPython/C++の最大差0、固定後228件の選択も全件一致した。振り分けまでの時間は平均約0.24〜0.25msで、初期化も含む。

GCC14.2/C++23、LOCALなし、親の1.9秒基準、2並列、各入力各版1回。公式Rust visと独立Python再生で、本測定2,320出力すべて合法、全帰巣、手数/公式スコア一致、診断エラー0を確認した。v320は新規保留とinの全228件で外部2秒以内、最大1.931234秒だった。ただし223件が外部1.9秒を超え、厳密な全件外部1.9秒以内は未達である。Python側の起動と終了待ちを含む外部時間と、stderrの内部経過時間を分けて保存した。全版ではv113の3件が外部2秒超、最大2.047595秒で、削除や測り直しはしていない。

入力単位の95%区間は対v111の平均手数差がin約[-0.75,+1.04]手、新規保留約[-1.383,+0.477]手。再実行ノイズの区間ではない。安定した劣化と断定する比較ではないが、今回の採用条件は未達とする。

### 保存と次の前提

validation1の500件は今回実行せず、その評価値を学習ラベルにも使っていない。学習と保留は新規生成、inは条件固定後の確認に限った。validation1を重複検査にも開かなかったため、過去の全データとの完全な非重複を実測保証したとは扱わない。

完全CPP、復元入口、学習条件、評価結果と台帳をmainへ直接保存した。ソースSHA256は9230c3d0ee6ec5ad955f862b81eec70ce682b3f4087983631f67948ab60d23f8。新規入力のmanifest、教師CSV、全出力、stderr、学習器、診断、凍結資料は会話添付AHC072_v320_evaluation_bundle.zipへ保存する。一時Actionsは同一非公開repoの取得と記録だけに使い、solverと学習は実行しない。転送用payloadの未完と誤挿入だけを訂正し、全CSVの独立SHA256照合を通過してから記録した。評価済みsolverは変更していない。新しいブランチ、force push、AtCoder投稿、既存の学習と現行指定の変更、eval viewerと共有評価ログの更新は行っていない。

次の前提: 版ごとの事後最良に余地があっても、今回の初期17特徴と320教師の浅い木では固定v113への上乗せを確認できなかった。特徴不足、教師数、時間依存探索の変動の寄与は未分離である。再開条件は、別の特徴や教師設計を事前登録する新しい明示指示。今回の保留やinの値を使った追加調整は行わず停止する。
'''
p=ROOT/'notes/experiments/v320.md';old=p.read_text()
if '## 実験後' not in old:p.write_text(old.rstrip()+'\n'+POST)
entry='''- **[v320] 新規入力から既存版の割当を学ぶ** → [v320](experiments/v320.md) 不採用。validation1は実行せず、新規320入力×4版で深さ0/1/2を5分割比較し、同色の隣接個体を持つ割合10.574%以下ならv315、それ以外v113とした。条件固定後の新規保留128件は現行v111比58手減だが、in100は15手増・v800固定相対−0.153418ポイント。固定v113比では両集合で52/57手増となり、単純な振り分けの上乗せは確認できなかった。本測定2,320出力は公式採点と独立再生で合法かつ全帰巣。新CPPの最大外部時間1.931234秒、2秒超0、外部1.9秒超223/228。現行を維持し、保留やinからの条件変更は行わない。再開条件: 別の特徴または教師設計への新しい明示指示と事前登録。

'''
p=ROOT/'notes/backlog.md';text=p.read_text();marker='## 決着済み'
assert marker in text
if '**[v320] 新規入力から既存版の割当を学ぶ**' not in text:
    text=text.replace(marker,marker+'\n\n'+entry,1);p.write_text(text)
print('Verified and recorded v320:',len(rows),'paired input rows; zero solver runs')
