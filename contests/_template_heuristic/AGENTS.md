# Heuristic Contest Agent Notes

## 前提
- このディレクトリがプロジェクトルートである。親ディレクトリや兄弟ディレクトリには依存しない。
- 言語は Rust である。
- AtCoder のジャッジ環境を前提にし、現在の依存環境以外は用いない。
- 提出候補は `src/bin/*.rs` に複数保持してよい。提出時はそのファイルを直接使う前提で扱う。
- `src/bin/*.rs` は提出時にコピペできるよう、各ファイル単体で完結している必要がある。
- 問題文や要点は `problem_description.txt` に記録する。
- 公式配布物は `tools/` と `samples/` に配置する。
- visualizer 実装は `.agents/skills/make-visualizer/SKILL.md` に従う。

## 生成AI利用ルール (AtCoder Heuristic Contest 生成AI利用ルール の解釈ボーダー)
- AI は 1 つの会話で、新しく生成する解候補を 1 つまでにする。実務上は `src/bin` に増える新規ファイルを 1 本までとみなす。
  - なお、1つの会話とは、ユーザーの指示に対してAIが出力するまでである。次にユーザーの指示が入力されるときは新しい会話とみなす。そのため 1 threadのなかで複数のversionを作成することは問題ない。
- AI は現在取り組んでいる 1 候補に対しては、実行確認、デバッグ、改善をしてよい。
- AI が複数候補を生成し、テストケースで自動比較・自動選別しながら改善することは禁止する。
- 複数候補の比較は、人間が明示的に指定した既存候補に対してのみ行ってよい。

### OK例
- `この1案を改善して`
- `この bin を実行してバグを直して`
- `既存の v003 と v007 を比較して`

### NG例
- `3案新しく作って比較して`
- `10個生成して一番スコアの良いものを選んで`
- `改善案を複数作って自動でベンチを回し、良いものだけ残して`

## 採点運用ルール
- 評価は競技時間内のボトルネックになりやすいため、`score_tools.sh` ではケース単位で自動並列化する。
- 並列数は CPU 数から `cpu//2` を採用する（最小 1）。
- スクリプトは環境変数より実行ルールを内包し、ケース分割を前提にする。
- `results/score_summary.csv` はコンテスト時の要約ログ。評価イベント単位で必ず追記する前提とする。

## ディレクトリ構成
主要なものだけ示す。生成物ディレクトリ (`target/`, `node_modules/`, `dist/`, `wasm/target/`) は除く。

```text
_template_heuristic/
├── problem_description.txt
├── Cargo.toml
├── .agents/skills/make-visualizer/SKILL.md
├── src/bin/
├── scripts/
├── notes/
├── results/
│   ├── score_summary.csv
│   └── out/
├── samples/
├── tools/
├── src_vis/main.js
├── wasm/src/lib.rs
├── wasm/src/impl_vis.rs
└── public/wasm/
```

## 各ディレクトリ・ファイルの役割
- `problem_description.txt`
  - 問題文、制約、スコア、初動メモの保存先である。
- `src/bin/*.rs`
  - 実験コードと提出候補を置く場所である。
- `src/bin/v001_template.rs`
  - 最初の解法の叩き台である。
- `src/bin/crate_check.rs`
  - AtCoder 用 crate 群が解決できるかを確認するためのプログラムである。
- `scripts/`
  - 実行、採点、generator、tools 展開、WASM build、visualizer 起動を行う補助コマンド群である。
- `notes/`
  - 問題固有の発見や性質を記録する場所である。
- `results/score_summary.csv`
  - score の要約ログの蓄積先である。`bin,total_avg,avg_elapsed,total_sum,total_min,total_max,eval_set,total_cases` の順で追記する。
- `results/out/<bin_name>/...`
  - `scripts/run.sh` の入力付き実行で生成された出力ファイルの格納場所。`bin_name` ごとに分離される。
- `tools/`
  - 公式 generator / tester / scorer の配置先である。
- `samples/`
  - サンプル input / output の配置先である。
- `src_vis/main.js`
  - visualizer の UI とローカル API 連携を書く。
- `wasm/src/impl_vis.rs`
  - 問題固有の visualizer ロジック本体である。
- `public/wasm/`
  - `build_wasm.sh` の生成物が出る場所である。

## shell script の役割
- `scripts/run.sh`
  - `cargo run --release --bin <name>` を実行し、`bin/input/elapsed/score/output` を標準出力する。
- `scripts/score_tools.sh`
  - `tools` 側の scorer を呼び、ケースを `cpu//2` 並列で走査する。
  - `./scripts/score_tools.sh <bin_name>` は `tools/in` と `results/out/<bin_name>` を対応付けて採点し、要約を記録する。
- `scripts/gen_tools.sh`
  - `tools` 側の generator を呼ぶための薄い wrapper である。
- `scripts/unpack_tools.sh`
  - 公式配布 zip を `tools/` に展開する。
- `scripts/build_wasm.sh`
  - `wasm-pack` を使って browser 用の生成物を `public/wasm/` に出力する。
- `scripts/dev_vis.sh`
  - 必要なら `yarn install` を行ったうえで Vite 開発サーバーを起動する。

## AI が意識すること
- 人間向けの使い方や作業順は `README.md` にある。ここでは構造・制約・役割を優先して参照する。
- `tools/` の中身は contest ごとに異なる。wrapper script の引数や期待する bin 名は固定だと思い込まない。
- visualizer 実装に入る前に `problem_description.txt` と `tools/src/` の存在を確認する。
- `public/wasm/` は手書きではなく build 生成物の置き場として扱う。
