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
│   └── scores.csv
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
- `README.md`
  - 人間向けの使い方を書く。
- `AGENTS.md`
  - AI に知っておいてほしい前提、制約、構成を書く。

### 解法・実験
- `src/bin/`
  - 解法の各バージョンを置く。提出時はこの中のファイルを直接使う。
- `src/bin/v001_template.rs`
  - 最初の実装の叩き台である。
- `src/bin/crate_check.rs`
  - AtCoder で使える Rust crate 群が解決できるか確認するためのプログラムである。
- `results/scores.csv`
  - 実行ログを追記する。比較の履歴置き場である。
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
- `public/wasm/`
  - `build_wasm.sh` の生成物が置かれる。

## 基本的な使い方
### 最初にやること
1. `problem_description.txt` を埋める
2. 公式配布物を `tools/` と `samples/` に置く
3. `src/bin/v001_*.rs` のようなファイルを作って実験を始める

### いつもの流れ
1. `src/bin/*.rs` に解法の各バージョンを書く
2. `./scripts/run.sh <bin_name> [input_file] [score]` で試し、`results/scores.csv` に記録する
3. 公式ツールが `vis` を持つ場合は `./scripts/vis_tools.sh ...`、`score` を持つ場合は `./scripts/score_tools.sh ...` で公式スコアを確認する
4. 複数ケースをまとめて回すときは `./scripts/bench.sh <bin_name> [input_dir]` を使う
5. 良さそうなバージョンが決まったら `./scripts/promote.sh <bin_name>` で提出候補としてビルド確認する
6. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

## shell script の役割
- `./scripts/run.sh <bin_name> [input_file] [score]`
  - Rust bin を実行し、`results/scores.csv` に `timestamp,bin,input,elapsed_sec,score` を追記する。
- `./scripts/vis_tools.sh <input_file> <output_file> [html_output_path]`
  - 公式 `tools` の `vis` バイナリをラップし、標準出力にスコアを出しつつ HTML を保存する。既定の保存先は `results/vis.html` である。
- `./scripts/score_tools.sh <args...>`
  - 公式 `tools` の `score` バイナリをラップする。`score` が無く `vis` だけある contest では `vis_tools.sh` に自動転送する。
- `./scripts/bench.sh [--jobs N] <bin_name> [input_dir]`
  - `cargo build --release --bin <bin_name>` を 1 回だけ行い、複数ケースに対して並列実行・採点する。結果は `results/bench/*.csv` と `results/bench/*_summary.txt`、`results/score_summary.csv` に保存する。
- `./scripts/gen_tools.sh <args...>`
  - 公式 `tools` の `gen` バイナリをラップする。追加入力生成用である。
- `./scripts/promote.sh <bin_name>`
  - 指定した `src/bin/<bin_name>.rs` を release + offline でビルドし、提出候補として壊れていないか確認する。
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
./scripts/vis_tools.sh ./tools/in/0000.txt ./results/out/0000.txt
./scripts/score_tools.sh ./tools/in/0000.txt ./out/0000.txt
./scripts/bench.sh --jobs 8 v400 ./tools/in
./scripts/gen_tools.sh ./tools/seeds.txt --dir ./tools/in_extra
./scripts/promote.sh v001_template
./scripts/unpack_tools.sh
./scripts/unpack_tools.sh ./tools/u5OpcTjC.zip
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

## 補足
- `src/bin/*.rs` は提出時にそのまま使えるよう、1 ファイルで完結させる。
- `notes/` は contest 固有の内容に限定し、テンプレートの使い方はこの `README.md` にまとめる。
- 生成物は `target/`, `node_modules/`, `dist/`, `wasm/target/` に出る。
