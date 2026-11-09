# AHC074の保存と復習

最終提出と代表解法、振り返りを保存した。詳細な研究履歴は[元のAHC074リポジトリ](https://github.com/masakiaota/AHC074)に残す。選別と検証の記録は[MIGRATION.md](MIGRATION.md)を参照する。

## 読む順序

1. [問題文](problem_description.txt)、[記号一覧](notes/notations.md)、[重要な性質](notes/important_properties.md)で問題を確認する。
2. [解法ガイド](notes/solution-guide.html)でv5223の構成を理解する。図はHTMLをブラウザーで開くと操作できる。
3. [最終提出](src/bin/v5849_pressure_sparse_frontier_budget1980_local.cpp)と[提出結果](notes/status.md)を確認する。
4. [振り返り](REFRECTION.md)と、その参照先の実験記録を読む。

## 保存した解法

| 版 | 保存した理由 |
| --- | --- |
| v0000 | 問題のルールと共通土台 |
| v5191 | 解法ガイドが参照する提出版 |
| v5223 | 解法ガイドの基準 |
| v5333 | 解法ガイドが受注処理の構造を照合した版 |
| v5737 | 親となる配置を増やし、本番でも改善した版 |
| v5776 | 最終提出前の比較基準 |
| v5849の1.980秒版 | 最終提出原稿 |
| v6829 | 解法ガイドの操作例の生成元 |

各原稿はsrc/binに元の名前で置いた。v5849の実験ノートは元版の記録であり、1.980秒版の測定値は[専用の要約](results/v5849_pressure_sparse_frontier_budget1980_local_summary.json)を参照する。

## 実行する

GCC 15、Python 3、Rustを使う。コマンドはこのディレクトリで実行する。

```sh
cp config/local_profiles.example.json config/local_profiles.json
# hostname、local_time_ratio、eval_jobsを自分の端末に合わせて編集する。
python3 scripts/eval.py v5849_pressure_sparse_frontier_budget1980_local --dry-run -j 1
```

配布入力100件をtools/inに保存した。端末設定がない場合は時間係数1、1並列になる。保存済みのMac測定は係数0.8であり、異なる設定の時間と得点は同条件の測定として扱わない。LOCALなしの提出相当ビルドは次のコマンドで作る。

```sh
scripts/build_solver.sh --no-local v5849_pressure_sparse_frontier_budget1980_local
```

固定1,000件は保存した種と指紋から生成できる。

```sh
python3 adhoc/scripts/gen_in_1000.py
```

## 可視化する

Node.js、Corepack、wasm-packを用意し、次を実行する。依存関係とWASMは初回に生成する。

```sh
scripts/dev_vis.sh
```

表示されたURLを開くとケース可視化、`/eval.html`を開くと評価一覧を見られる。選んだ8本に該当する過去の評価行を保存した。ケース別出力は省略したため、過去の操作列を再生するには再実行または元ディレクトリの出力が必要である。解法ガイドの図はHTMLに埋め込まれており、過去の出力なしで読める。
