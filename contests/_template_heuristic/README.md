# Heuristic Contest Template

このディレクトリは、AtCoder Heuristic Contest 用の作業テンプレートである。  
解法実装、実験、採点、visualizer をこのディレクトリの中だけで進めることを想定している。

## 最初にやること
1. `problem_description.txt` に問題文と要点を書く
2. 公式配布物を `tools/` に展開する
3. `src/bin/v001_*.rs` のようなファイルを作って実験を始める

## 基本的な流れ
1. `src/bin/*.rs` に解法の各バージョンを書く
2. `./scripts/run.sh <bin_name> [input_file] [score]` で実行し、`results/scores.csv` に記録する
3. 採点器がある場合は `./scripts/score_tools.sh ...` でスコアを確認する
4. 提出候補が決まったら `./scripts/promote.sh <bin_name>` で offline ビルド確認を行う
5. 提出時は対象の `src/bin/<bin_name>.rs` をそのままコピーして使う

## よく使うコマンド
```bash
./scripts/run.sh v001_template
./scripts/run.sh v001_template ./tools/in/0000.txt
./scripts/score_tools.sh ./tools/in/0000.txt ./out/0000.txt
./scripts/promote.sh v001_template
./scripts/unpack_tools.sh ./tools.zip
./scripts/build_wasm.sh
./scripts/dev_vis.sh
```

## ディレクトリ
- `src/bin/`
  - 解法の各バージョンと提出候補を置く
- `tools/`
  - 公式 tester や generator を置く
- `results/`
  - 実行ログを置く
- `notes/`
  - 問題固有のアイデアや性質のメモを書く
- `wasm/`
  - visualizer の Rust 実装を置く
- `public/wasm/`
  - `./scripts/build_wasm.sh` の生成物が出る

## Visualizer
- ベース UI は `index.html`, `src_vis/main.js`, `vite.config.js`, `wasm/` に入っている
- 開発サーバーは `./scripts/dev_vis.sh`
- WASM を更新したら `./scripts/build_wasm.sh` を実行する
- 問題固有実装に入る前に `problem_description.txt` と `tools/src/` を揃える

## 補足
- `src/bin/*.rs` は提出時にそのまま使えるよう、1 ファイルで完結させる
- `notes/` は contest 固有の内容に限定し、テンプレートの使い方はこの `README.md` にまとめる
