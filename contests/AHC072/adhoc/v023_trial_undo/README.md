# v023: B表現と差分復元の比較

## 結論

v019〜v021の `Constructor::planPair` に、盤面全体をコピーして短い経路を試す箇所がある。ここを変更前の2マスの保存・復元へ置き換えたが、A/Bどちらにも明確な速度の利点は見えなかった。今回のB表現と差分復元の組み合わせは不採用とする。

| 方式 | 候補処理全体のCPU時間 | Aコピーとの差 |
| --- | ---: | ---: |
| A・コピー | 83.723 ms | 基準 |
| A・差分復元 | 83.820 ms | 0.116%増 |
| B・コピー | 84.547 ms | 0.984%増 |
| B・差分復元 | 83.584 ms | 0.166%減 |

1,561候補を処理する時間の8巡中央値。各方式の巡回間の幅はこの差より大きく、B差分復元が速いとする根拠は得られなかった。事前の採用基準は、B差分復元がAコピー・A差分復元の両方より5%以上短時間になることだった。

全100ケースの評価は合法・全帰巣・エラー0だったが、保存済みv021より49手（0.2339%）増えた。1回の時間制限下の比較であり、この差を恒常的な性能差とは扱わない。事前登録と判断の詳細は [v023実験ノート](../../notes/experiments/v023.md) に記録した。

## 測ったもの

- Aは色を1〜12として32 bitに格納する。Bは色を0〜11として32 bitに格納し、高さと向きを1 byteで持つ。Bの高さ参照では保持した値を使う。
- 変更対象は第三のマスへの合流試行である。確定盤面とDPのキーはAのままで、Bへは候補ごとに最初の試行時だけ変換する。この変換も計測に含める。
- 差分復元は各操作の変更前の2マスを記録し、逆順に書き戻す。帰巣で消えた色も復元できる。保存領域は再利用し、例外による中断でも復元する。
- 通常99件の保存済みv021解の0、1/4、1/2、3/4時点、計396盤面で、既存のvariant=0の候補生成を行った。各盤面で優先度上位4候補まで、計1,561候補を使った。元実行の内部履歴を記録したデータではない。
- 候補の生成、照合、DPの初回計算は計測外とする。全方式を事前に実行してDPを温め、計測中にキャッシュの要素数が増えていないことを確認した。
- `planPair` 全体を測定し、コピー・変換・操作・復元に加えて経路探索と候補比較も含めた。8巡で測定順を交代し、99件分のCPU時間を巡回ごとに合計して中央値を取った。CPU時間は `CLOCK_THREAD_CPUTIME_ID`、実時間は `steady_clock` を使う。
- Apple M3、GCC 15.1.0、C++23、既定-O2に親solverのO3 pragmaを保持した。小さなB操作はインライン展開を指定している。前回のv000の-O2比較とはコンパイル条件と対象処理が異なる。
- ベンチマークは非LOCAL相当、通常評価は既定LOCAL・並列数2でそれぞれ1回実行した。評価用ロックで別会話の評価と直列化した。

## 検証

4方式の8,115試行・22,254操作で、操作後の盤面、復元後の盤面、経路距離、経路、候補評価と選択された操作列が一致した。v023の本処理もAコピーの参照処理と一致した。塔の境界条件26,112件、保存済み解20,910操作の逐次再生、例外時復元99件にも成功した。

通常評価では、Bへの変換121,469回、差分復元の試行784,206回、保存した操作2,915,993回を確認した。独立再生による出力検証は全100件成功、最大実行時間は1,619 msだった。評価後にsolverとベンチマークの実装は変更していない。

## ファイル

- `samples.csv`: ケース・巡回・方式ごとのCPU時間、実時間、結果のチェック値。
- `checks.json`: 操作と候補の一致検証件数、盤面と保存要素の容量。
- `fixed_work_summary.json`: 4方式の巡回ごとの合計時間と中央値。
- `summary.json`: 固定仕事量と全100件の結果、独立再生、親との比較、採否。
- `source.diff`: v021からv023へのソース差分。
- `build_*.log`, `benchmark.log`, `evaluation.log`: ビルドと実行の記録。
- `../bin/bench_v023_trial_undo.cpp`: 4方式の比較と検証。
- `../scripts/summarize_v023.py`: 保存結果だけを読み、独立検証と集計を行うスクリプト。

## 再現用コマンド

以下は実行済み手順の記録であり、再評価を要求するものではない。ベンチマークはsolver内の非公開の候補生成処理を使用するため、補助コードだけ `-fno-access-control` を指定する。両者の実行時には `results/.eval.lock` を使用する。

```sh
SDKROOT="$(xcrun --show-sdk-path)" MACOSX_DEPLOYMENT_TARGET=15.0 \
g++-15 -std=gnu++23 -O2 -Wall -Wextra -march=native -pthread \
  -ftrivial-auto-var-init=zero -fopenmp -fno-access-control \
  -DATCODER -DONLINE_JUDGE -DNOMINMAX \
  adhoc/bin/bench_v023_trial_undo.cpp -o target/release/bench_v023_trial_undo

target/release/bench_v023_trial_undo adhoc/v023_trial_undo/samples.csv \
  > adhoc/v023_trial_undo/checks.json 2> adhoc/v023_trial_undo/benchmark.log

SDKROOT="$(xcrun --show-sdk-path)" MACOSX_DEPLOYMENT_TARGET=15.0 \
python3 scripts/eval.py --wait-lock --label lazy_trial_undo v023_lazy_trial_undo

python3 -B adhoc/scripts/summarize_v023.py
```
