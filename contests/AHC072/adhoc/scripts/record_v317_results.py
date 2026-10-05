#!/usr/bin/env python3
"""Record completed v317 evidence only. Never runs a solver or registers evals."""
from pathlib import Path
import base64,hashlib,zlib
ROOT=Path(__file__).resolve().parents[2]
SOURCE_SHA='5bd46f9b88d3e372ae1fb22d6f213a9188a1624ae0b54c3982773a861d57a39f'
CSV_SHA='7150d8e3ff6e2dfa124d69933b17404116c76b6cceb0109cf959ff7423279f05'
DATA='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'
POST='''### 判定

混合標準は不採用。inはv401比24手増で絶対合計非悪化の条件を満たさず、補助100件も47手増だった。現行v401を維持する。平均195手以下も未達。アブレーションの重み単独へ実測後に標準版を差し替えず、二つの条件のまま記録する。絶対値は[評価要約](../../adhoc/v317/evaluation_summary.json)、ケース別は[独立比較CSV](../../adhoc/v317/paired_scores.csv)に保存する。

重み単独版はinで178手減、固定参照相対+0.731107ポイント、52勝11分37敗だった。一方、補助100件は79手増、34勝12分54敗で、改善方向が一致しなかった。混合版のin相対は+0.075543ポイント、43勝11分46敗、補助は42勝15分43敗だった。この単独版を一般的な改善として採用する根拠にはしない。

### 機構と内訳

39,808局面、17,233,628個のlogitで、全量参照と差分推論の最終層、価値、logitがbit一致し、最大誤差0、最大logit選択の不一致0だった。固定時計64条件の操作列と残存数を照合し、重み切替500条件、強制無効化2,002回、帰巣時の全量更新3,560回を確認した。重み、LNS、CNN、育成のソースとLOCAL/非LOCALの完全前処理を確認した。追加学習や再量子化はない。

混合版はin96件で学習済みモデルの候補を保持し、46件でそのモデルが育成後の勝者となった。補助では95件と49件。モデル切替の平均時間はinで約0.153ms。初期最短列はinで混合254手減、単独565手減だったが、最終差はそれぞれ24手増、178手減となった。

混合版の原方向NN列のハッシュは親と全200件で一致した。0005では追加生成の時計判定により混合は旧モデル2本のみ、親は3本を保持し、混合は71手増だった。このケースでは新しいモデルを一度も使っておらず、悪化をモデルの質に帰属しない。0005以外の99件は混合47手減、単独196手減。単独の最大改善1件を除いても144手減だった。事後の分解であり、採否基準やモデル割当に反映していない。

### 検証と限界

in100とvalidation1から事前に選んだ補助100件で3条件を比較した。GCC14.2、LOCALなし、親と同じ1.9秒予算、2並列、各入力1回。全600出力が独立Python/C++で合法かつ全帰巣し、T/E/Sが一致した。公式Rustバイナリは実行していない。補助は既に評価した集合で新規未見ではない。

外部1.9秒超は親3件、重み単独7件、混合1件。最大は親2.023430秒、重み単独1.937506秒、混合1.912905秒。時間式は親を無変更で保持したが、厳密な全件外部1.9秒以内は達成していない。入力bootstrap95%区間はin平均差が混合[-1.27,+2.19]手、単独[-3.27,-0.38]手。これは入力構成の区間で、再実行ノイズを含まない。

### 保存と次の前提

完全なCPP、復元入口、差分、独立評価要約とCSVをmainへ直接保存した。CPPのSHA256は5bd46f9b88d3e372ae1fb22d6f213a9188a1624ae0b54c3982773a861d57a39f。V317_LEARNED_ONLYの比較入口はadhoc/bin/v317_learned_only.cppである。会話添付の単独CPPはその機械的展開で、両モードの完全前処理一致を確認したが展開版を再実行していない。全600出力、入力、ハッシュ、診断、再現コードはAHC072_v317_evaluation_bundle.zipに保存する。

直接git通信はDNSエラーだったため、接続済みGitHubと同じ非公開repoの一時Actionsで取得とhash検証済みCPPの記録だけを行った。Actionsでsolverや学習は実行していない。転送用スクリプトの誤記だけ修正し、評価済みsolverは不変。新しいブランチ、eval viewerへの登録、共有評価CSV/JSONL更新、AtCoder提出は行っていない。

次の前提: 報酬が違うNNの混合は動作し、計画の多様化も起きたが、今回の固定割当では両集合の最終手数を改善しなかった。新重みだけのin改善も補助では逆転した。再開条件: ユーザー環境で同じ固定二条件を比較するか、モデル割当、候補数、育成を切り分ける新しい具体的仮説への明示指示。今回の結果を見てのsolver調整、追加評価、重み差し替えは行わない。
'''
ENTRY='2026-10-04 v317外部比較: v401とv109固定重みを最大4初期解内で混ぜて評価した。混合はin24手増・相対+0.075543ポイント、補助100件47手増で不採用。事前固定した全候補v109重み条件はin178手減・相対+0.731107ポイントだが補助79手増。全600出力合法・全帰巣、厳密な全件外部1.9秒は未達。主対照と現行はユーザー指定v401。詳細は[v317](experiments/v317.md)。Studio256件の元案をこの外部200件比較の結果と同一視しない。再開は同一固定条件のユーザー環境比較または割当を切り分ける新指示による。'
assert hashlib.sha256((ROOT/'src/bin/v317_complementary_policies.cpp').read_bytes()).hexdigest()==SOURCE_SHA
csv=zlib.decompress(base64.b64decode(DATA));assert hashlib.sha256(csv).hexdigest()==CSV_SHA
out=ROOT/'adhoc/v317/paired_scores.csv'
if out.exists():assert out.read_bytes()==csv
else:out.write_bytes(csv)
note=ROOT/'notes/experiments/v317.md';text=note.read_text()
if POST not in text:
    marker='## 実験後\n\n未実行。\n'
    assert text.count(marker)==1,'Do not overwrite concurrent results'
    note.write_text(text.replace(marker,'## 実験後\n\n'+POST))
backlog=ROOT/'notes/backlog.md';text=backlog.read_text()
if ENTRY not in text:
    lines=text.splitlines(True);positions=[i for i,line in enumerate(lines) if line.startswith('- **[B-168] ')]
    assert len(positions)==1,'B-168 was changed concurrently; review explicitly'
    i=positions[0];old=lines.pop(i).rstrip('\n')
    at=next(i for i,line in enumerate(lines) if line.rstrip()=='## 決着済み')+1
    lines.insert(at,'\n'+old+'\n\n  '+ENTRY+'\n\n')
    backlog.write_text(''.join(lines))
print('v317 recorded; no solvers executed; no viewer registration')
