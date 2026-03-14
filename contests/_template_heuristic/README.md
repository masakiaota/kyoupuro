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
│       └── make-visualizer/
│           └── SKILL.md
├── src/
│   └── bin/
│       ├── v001_template.rs
│       └── crate_check.rs
├── scripts/
├── notes/
├── results/
│   ├── score_summary.csv
│   └── out/
├── samples/
├── tools/
├── src_vis/
├── wasm/
└── public/
    └── wasm/
```

## 役割
### ルート
- `problem_description.txt`
  - 問題文、入出力、スコア、制約、初動メモを書く。

### 解法・実験
- `src/bin/`
  - 解法の各バージョンを置く。提出時はこの中のファイルを直接使う。
- `results/score_summary.csv`
  - score の要約ログを追記する。評価イベント単位で集約結果を残す。
- `results/out/<bin_name>/`
  - `run.sh` 実行時の出力を保存する。`bin` ごとにフォルダ分けされる。
- `notes/`
  - 問題固有のアイデア、重要な性質、観察結果を書く。

### contest assets
- `tools/`
  - 公式 generator / tester / scorer を展開する場所である。
- `samples/`
  - サンプル input / output を置く場所である。

### visualizer
- `.agents/skills/make-visualizer/SKILL.md`
  - visualizer 実装時に AI が従う手順である。
- `src_vis/main.js`
  - Vite 側の UI ロジックとローカル API 連携を書く。
- `wasm/src/impl_vis.rs`
  - 問題固有の visualizer 実装本体である。

## 基本的な使い方
### 最初にやること
1. `problem_description.txt` を埋める
2. 公式配布物を `tools/` と `samples/` に置く
3. 並列評価できるように `scripts/score_tools.sh` を編集。
4. 必要なら visualizer もつくる (適宜改善)。
5. `src/bin/v001_*.rs` のようなファイルを作って実験を始める

### いつもの流れ
1. `src/bin/*.rs` に解法の各バージョンを書く
2. `./scripts/run.sh <bin_name> [input_file] [score]` で試し、結果を標準出力で確認する  
   - `input_file` 指定時は出力を `results/out/<bin_name>/<input_file_basename>` に保存する。
3. scorer があるなら `./scripts/score_tools.sh` で公式スコアを確認する
   - デフォルトは `tools/in` と `tools/out` を使い、ケース単位で `score_tools.sh` が並列実行する
4. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

## shell script の役割
- `./scripts/run.sh <bin_name> [input_file] [score]`
  - Rust bin を実行し、`bin=..., input=..., elapsed=..., score=..., output=...` を標準出力する。
- `./scripts/score_tools.sh`
  - 公式 `tools` の `score` バイナリをラップする。
  - `./scripts/score_tools.sh <bin_name>` で、`tools/in` と `results/out/<bin_name>` の対応で一括採点する。
  - 単発: `./scripts/score_tools.sh <input_file> <output_file>`
  - ディレクトリ指定: `./scripts/score_tools.sh <input_dir> <output_dir>`
  - `bin_name` 指定版: `./scripts/score_tools.sh <bin_name> <input_file> <output_file>` / `./scripts/score_tools.sh <bin_name> <input_dir> <output_dir>`
  - 入力/出力を多数持つ場合は `tools/in` と `tools/out`、または `results/out/<bin_name>` の対応ペアを自動検出する。並列数は `cpu//2`。
  - 要約は `results/score_summary.csv` に `bin,total_avg,avg_elapsed,total_sum,total_min,total_max,eval_set,total_cases` で追記される。
- `./scripts/gen_tools.sh <args...>`
  - 公式 `tools` の `gen` バイナリをラップする。追加入力生成用である。
- `./scripts/unpack_tools.sh [tools_zip_path]`
  - `tools.zip` などの公式配布 zip を `tools/` に展開する。
- `./scripts/build_wasm.sh`
  - `wasm-pack build --target web --out-dir ../public/wasm` を実行し、browser 用 WASM を更新する。
- `./scripts/dev_vis.sh`
  - 必要なら `yarn install` を行い、Vite の開発サーバーを起動する。

## よく使うコマンド
```bash
./scripts/run.sh v001_template
./scripts/run.sh v001_template ./tools/in/0000.txt
./scripts/score_tools.sh v001_template ./tools/in/0000.txt ./tools/out/0000.txt
./scripts/score_tools.sh ./tools/in/0000.txt ./tools/out/0000.txt
./scripts/score_tools.sh
./scripts/score_tools.sh v001_template
./scripts/unpack_tools.sh ./tools.zip
./scripts/build_wasm.sh
./scripts/dev_vis.sh
cargo run --bin crate_check
```

## Visualizer の使い方
- まず `problem_description.txt` と `tools/src/` を揃える
- `wasm/src/impl_vis.rs` に問題固有の描画ロジックを入れる
- `./scripts/build_wasm.sh` で `public/wasm/` を更新する
- `./scripts/dev_vis.sh` でローカル server を立ち上げる
- `src_vis/main.js` には Rust bin 実行 UI と SVG 表示 UI が入っている
