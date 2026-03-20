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
  - `run.sh` や `eval.sh` 実行時の出力を保存する。`bin` ごとにフォルダ分けされる。
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
3. 並列評価できるように `scripts/eval.sh` が contest の scoring tool 呼び出し方に対応するように編集。
4. 必要なら visualizer もつくる (適宜改善)。
5. `src/bin/v001_*.rs` のようなファイルを作って実験を始める

### 実験の流れ
1. `src/bin/*.rs` に解法の各バージョンを書く
2. `./scripts/run.sh <bin_name> [input_file]` で単発確認する
   - `input_file` 指定時は `results/out/<bin_name>/<input_file_basename>` に出力を保存する。
3. scorer があるなら `./scripts/eval.sh [-v] <bin_name> [input_dir]` で公式スコアを確認する
   - `input_dir` 省略時は `tools/in` を使う。
   - 各ケースについて `run -> score` を `cpu//2` 並列で実行する。
   - 出力は `results/out/<bin_name>/` に保存し、要約は `results/score_summary.csv` に追記する。
4. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

## shell script の役割
- `./scripts/run.sh <bin_name> [input_file]`
  - stdin または 1 つの input_file に対して手動実行する。
- `./scripts/eval.sh [-v] <bin_name> [input_dir]`
  - solver と公式 `score` を 1 回だけ build し、ケース単位で `run -> score` を `cpu//2` 並列実行する。
  - 出力は `results/out/<bin_name>/` に保存し、要約は `results/score_summary.csv` に追記する。
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
./scripts/eval.sh v001_template
./scripts/eval.sh -v v001_template
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
