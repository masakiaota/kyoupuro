# Heuristic Contest Template

このディレクトリは、AtCoder Heuristic Contest 用の作業テンプレートである。  
解法実装、実験、採点、visualizer をこのディレクトリの中だけで進める前提で作ってある。

## ディレクトリ構成
主要なものだけ示す。

```text
_template_heuristic/
├── README.md
├── AGENTS.md
├── problem_description.txt
├── Cargo.toml
├── .agents/
│   └── skills/
│       ├── write-problem-description/
│       │   └── SKILL.md
│       └── make-ahc-visualizer/
│           └── SKILL.md
├── src/
│   └── bin/
│       ├── v000_template.rs
│       └── crate_check.rs
├── scripts/
├── notes/
│   ├── important_properties.md
│   └── notations.md
├── results/
│   ├── score_summary.csv
│   ├── score_detail.csv
│   ├── eval_records.jsonl
│   └── out/
├── samples/
└── tools/
```

## 役割
### ルート
- `problem_description.txt`
  - 問題文、入出力、スコア、制約、初動メモを書く。
- `.agents/skills/write-problem-description/SKILL.md`
  - `problem_description.txt` 作成時に AI が従う手順である。

### 解法・実験
- `src/bin/v000_template.rs`
  - 問題理解の過程で確定した共通土台を置く。
  - 例: `State`、問題のルール再現、基本遷移、制約判定、整合性チェック、reference 実装。
- `src/bin/v001_*.rs` 以降
  - 探索戦略、評価関数、パラメータ、枝刈りなど、試行錯誤する solver を置く。提出時はこの中のファイルを直接使う。
- `results/score_summary.csv`
  - score の要約ログを追記する。列順は `bin,total_avg,total_sum,total_min,total_max,avg_elapsed,max_elapsed,eval_set,total_cases,label,executed_at` である。
- `results/score_detail.csv`
  - `tools/in` 専用の wide-format 比較表である。列順は `bin,total_avg,max_elapsed,<case_name_1>,...,label,executed_at` である。
- `results/eval_records.jsonl`
  - 1 行 1 case の評価記録を追記する正本である。`score_detail.csv` や比較表示の材料にする。
- `results/out/<bin_name>/`
  - `run.sh` や `eval.py` 実行時の出力を保存する。`bin` ごとにフォルダ分けされる。
- `notes/`
  - 問題固有のアイデア、重要な性質、観察結果を書く。
- `notes/notations.md`
  - 問題で使う記号と、コード上の代表名・型・制約をまとめる正本である。
- `notes/important_properties.md`
  - 問題から導かれる重要な性質、不変量、探索や構築で効く性質を整理する正本である。

### contest assets
- `tools/`
  - 公式 generator / tester / scorer を展開する場所である。
- `samples/`
  - サンプル input / output を置く場所である。

### visualizer
- `.agents/skills/make-ahc-visualizer/SKILL.md`
  - visualizer 実装時に AI が従う手順である。UI / WASM / Vite のテンプレートは skill の同梱物から project root に展開する。

## 基本的な使い方
### 最初にやること
1. `.agents/skills/write-problem-description/SKILL.md` に従って `problem_description.txt` を埋める
2. 公式配布物を `tools/` と `samples/` に置く
3. 並列評価できるように `scripts/eval.py` が contest の scoring tool 呼び出し方に対応するように編集。
4. 必要な記号を `notes/notations.md` に早めに書き出し、命名と型の正本を固める。
5. 見えてきた重要な性質や不変量を `notes/important_properties.md` に整理する。
6. 必要なら `.agents/skills/make-ahc-visualizer/SKILL.md` に従って visualizer を作る。
7. `src/bin/v000_template.rs` に共通土台を整え、実験用 solver は `v001_*.rs` 以降として追加する

### 実験の流れ
1. 共通土台は `src/bin/v000_template.rs` に書き、試行錯誤する solver は `src/bin/v001_*.rs` 以降に書く
2. `./scripts/run.sh <bin_name> [input_file]` で単発確認する
   - `input_file` 指定時は `results/out/<bin_name>/<input_file_basename>` に出力を保存する。
3. scorer があるなら `./scripts/eval.py [-v] [-j jobs] [--label label] [--dry-run] <bin_name> [input_dir]` で公式スコアを確認する
   - `input_dir` 省略時は `tools/in` を使う。
   - build 後に先頭入力で `run -> score` を 1 回ウォームアップしてから、各ケースについて本番の `run -> score` を `cpu//2 - 1` 並列で実行する。最小値は 1 である。
   - ウォームアップ結果は score ログ、elapsed 集計、本番出力には含めない。
   - 発熱や計測ぶれを避けたいときは `-j 1` で直列実行する。
   - 通常実行では `results/score_summary.csv` と `results/eval_records.jsonl` に追記し、`tools/in` を全ケース成功で評価したときだけ `results/score_detail.csv` にも追記する。
   - `--dry-run` は蓄積ファイルを更新せず、その場の確認だけを行う。
4. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

## script の役割
- `./scripts/run.sh <bin_name> [input_file]`
  - stdin または 1 つの input_file に対して手動実行する。
- `./scripts/eval.py [-v] [-j jobs] [--label label] [--dry-run] <bin_name> [input_dir]`
  - solver と公式 `score` を 1 回だけ build し、先頭入力で `run -> score` を 1 回ウォームアップしてから、ケース単位で本番の `run -> score` を実行する。
  - ウォームアップ結果は score ログ、elapsed 集計、本番出力には含めない。
  - 既定ジョブ数は `cpu//2 - 1` で、最小値は 1 である。`-j 1` で直列評価に切り替えられる。
  - 出力は `results/out/<bin_name>/` に保存し、要約は `results/score_summary.csv` に追記する。`tools/in` を全ケース成功で評価したときだけ `results/score_detail.csv` にも追記し、全ケースの記録は `results/eval_records.jsonl` に追記する。
  - `--dry-run` は 3 つの蓄積ファイルを更新しない。
  - `-h` / `--help` で使い方を確認できる。
- `./scripts/gen_tools.sh <args...>`
  - 公式 `tools` の `gen` バイナリをラップする。追加入力生成用である。
- `./scripts/unpack_tools.sh [tools_zip_path]`
  - `tools.zip` などの公式配布 zip を `tools/` に展開する。

## よく使うコマンド
```bash
./scripts/run.sh <bin_name>
./scripts/run.sh <bin_name> ./tools/in/0000.txt
./scripts/eval.py <bin_name>
./scripts/eval.py -j 1 <bin_name>
./scripts/eval.py -v --label baseline <bin_name>
./scripts/eval.py --dry-run <bin_name>
./scripts/eval.py --help
./scripts/unpack_tools.sh ./tools.zip
cargo run --bin crate_check
```

## Visualizer の使い方
- まず `problem_description.txt` と `tools/src/` を揃える
- `.agents/skills/make-ahc-visualizer/SKILL.md` を読み、同梱テンプレートを project root に展開する
- skill の指示に従い、問題固有部分だけを編集して起動確認する
