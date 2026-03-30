# Heuristic Contest Agent Notes

## 前提
- このディレクトリが project root である。親や兄弟ディレクトリには依存しない。
- 言語は Rust のみである。
- AtCoder のジャッジ環境を前提にし、現在の依存環境以外は用いない。
- `src/bin/*.rs` には複数の候補を置いてよい。各ファイルは単体で完結し、1 行目に `// <file_name>.rs` を置く(AtCoder上でも識別できるように)。
- `src/bin/v000_template.rs` はコピー元のテンプレートである。実験用の解法は通常 `v001_*.rs` から作り始める。
- わからないことに関しては(特に問題の考察に関して)、それっぽい解説をするのではなく「わからない」と認める。
- 問題文や要点は `problem_description.txt` に記録する。
- 公式配布物は `tools/` と `samples/` に配置する。
- visualizer実装は `.agents/skills/make-visualizer/SKILL.md` に従う。

## 生成AI利用ルール (AtCoder Heuristic Contest 生成AI利用ルール の解釈ボーダー)
- AI は 1 つの会話で、新しく生成する解候補を 1 つまでにする。実務上は `src/bin` に増える新規ファイルを 1 本までとみなす。
  - なお、1つの会話とは、ユーザーの指示に対してAIが出力するまでである。次にユーザーの指示が入力されるときは新しい会話とみなす。そのため 1 threadのなかで複数のversionを作成することは問題ない。
- AI は現在取り組んでいる 1 候補に対しては、実行確認、デバッグ、改善をしてよい。
- AI は、人間が指定した既存 solver に対するハイパーパラメータ探索スクリプトを作成してよい。
- AI が直接複数候補を生成し、テストケースで自動比較・自動選別しながら改善することは禁止する。
- 複数候補の比較は、人間が明示的に指定した既存候補に対してのみ行ってよい。

### OK例
- `この1案を改善して`
- `この bin を実行してバグを直して`
- `既存の v003 と v007 を比較して`

### NG例
- `3案新しく作って比較して`
- `10個生成して一番スコアの良いものを選んで`
- `改善案を複数作って自動でベンチを回し、良いものだけ残して`

## 評価運用ルール
- `scripts/run.sh` は単発の手動実行専用である。
- `scripts/eval.sh` は評価パイプライン本体である。solver と tools の score をそれぞれ 1 回だけ build し、その後は `run -> score` をケース単位で直列実行する worker を `cpu//2` 並列で回す。
- `results/out/<bin_name>/` は最新評価の scratch/workspace である。`eval.sh` 実行時に同名 basename の出力が並ぶ前提なので、重複 basename は拒否する。
- `results/score_summary.csv` は評価要約ログである。列順は `bin,total_avg,avg_elapsed,total_sum,total_min,total_max,eval_set,total_cases`。全ケース成功時のみ追記する。
- verbose は進捗表示だけに使い、追加ログを恒久保存しない。

## ディレクトリ構成
主要なものだけ示す。生成物ディレクトリ (`target/`, `node_modules/`, `dist/`, `wasm/target/`) は除く。

```text
_template_heuristic/
├── problem_description.txt
├── notes/
├── results/
│   ├── score_summary.csv
│   └── out/
├── samples/
├── tools/
├── src/
│   └── bin/
├── scripts/
├── src_vis/
├── wasm/
└── .agents/skills/make-visualizer/SKILL.md
```

## 各ディレクトリ・ファイルの役割
- `problem_description.txt`
  - 問題文、制約、スコア、初動メモの保存先である。
- `src/bin/*.rs`
  - 実験コードと提出候補を置く場所である。
- `scripts/run.sh`
  - 1 件の入力に対する手動実行を行う。
- `scripts/eval.sh`
  - 公式 scorer を使った並列評価本体である。
- `notes/`
  - 問題固有の発見や性質を記録する場所である。
- `results/score_summary.csv`
  - score 要約の蓄積先である。
- `results/out/<bin_name>/...`
  - `eval.sh` 実行時の出力ファイルの格納場所である。
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
  - `src/bin/<name>.rs` をビルドし、stdin か 1 つの input file を手動確認する。
- `scripts/eval.sh`
  - solver と score を 1 回だけ build し、ケース単位で `run -> score` を並列実行する。
  - `./scripts/eval.sh <bin_name>` は `tools/in` と `results/out/<bin_name>` を使う。
- `scripts/gen_tools.sh`
  - `tools` 側の `gen` バイナリを呼ぶための薄い wrapper である。
- `scripts/unpack_tools.sh`
  - 公式配布 zip を `tools/` に展開する。
- `scripts/build_wasm.sh`
  - `wasm-pack` を使って browser 用の生成物を `public/wasm/` に出力する。
- `scripts/dev_vis.sh`
  - 必要なら `yarn install` を行ったうえで Vite 開発サーバーを起動する。

## AI が意識すること
- `tools/` の中身は contest ごとに異なる。wrapper script の引数や期待する bin 名は固定だと思い込まない。
- visualizer 実装に入る前に `problem_description.txt` と `tools/src/` の存在を確認する。
- `public/wasm/` は手書きではなく build 生成物の置き場として扱う。
