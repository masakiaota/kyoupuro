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
3. scorer があるなら `./scripts/score_tools.sh ...` で公式スコアを確認する
4. 良さそうなバージョンが決まったら `./scripts/promote.sh <bin_name>` で提出候補としてビルド確認する
5. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

## shell script の役割
- `./scripts/run.sh <bin_name> [input_file] [score]`
  - Rust bin を実行し、`results/scores.csv` に `timestamp,bin,input,elapsed_sec,score` を追記する。
- `./scripts/score_tools.sh <args...>`
  - 公式 `tools` の `score` バイナリをラップする。具体的な引数は contest ごとの配布物に合わせる。
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
./scripts/score_tools.sh ./tools/in/0000.txt ./out/0000.txt
./scripts/promote.sh v001_template
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

## 補足
- `src/bin/*.rs` は提出時にそのまま使えるよう、1 ファイルで完結させる。
- `notes/` は contest 固有の内容に限定し、テンプレートの使い方はこの `README.md` にまとめる。
- 生成物は `target/`, `node_modules/`, `dist/`, `wasm/target/` に出る。
