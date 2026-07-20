---
name: make-ahc-visualizer
description: AHC形式のRustプロジェクトで、problem_description.txt と公式 tools/src をもとに、同梱テンプレートから case visualizer / eval viewer / WASM 接続を完成させる。
---

# make-ahc-visualizer

AHC形式の visualizer を作る。UI/API/eval viewer の大枠は同梱テンプレートをコピーし、問題固有の接続だけを編集する。

## 1. 前提条件チェック

project root は `AGENTS.md` または `README.md` があるディレクトリである。作業前に以下を確認し、不足があれば停止してユーザーに報告する。

- `problem_description.txt` が存在し、プレースホルダーではない
- `tools/src/` が存在する
- `Cargo.toml` が存在する
- `corepack`, `wasm-pack`, `cargo` が実行できる

## 2. テンプレート配置

この skill の `assets/template/` を project root にコピーする。既存ファイルがない前提であるため、もし存在する場合は一時停止してユーザーに確認する。

例:

```sh
SKILL_DIR=".agents/skills/make-ahc-visualizer"
cp "$SKILL_DIR/assets/template/index.html" .
cp "$SKILL_DIR/assets/template/eval.html" .
cp "$SKILL_DIR/assets/template/vite.config.js" .
cp "$SKILL_DIR/assets/template/package.json" .
mkdir -p src_vis src_eval wasm/src scripts
cp "$SKILL_DIR/assets/template/src_vis/main.js" src_vis/main.js
cp "$SKILL_DIR/assets/template/src_eval/"*.js src_eval/
cp "$SKILL_DIR/assets/template/wasm/Cargo.toml" wasm/Cargo.toml
cp "$SKILL_DIR/assets/template/wasm/src/"*.rs wasm/src/
cp "$SKILL_DIR/assets/template/scripts/"*.sh scripts/
chmod +x scripts/build_wasm.sh scripts/dev_vis.sh
```

`src_vis/wasm/` は build 生成物なので手書きしない。

## 3. 問題理解

以下を読む。

1. `problem_description.txt`
2. `tools/src/lib.rs`
3. 必要なら `tools/src` の非 `bin` モジュール

`tools/src/bin/` は原則として設計根拠にしない。公式 `tools/src` に parse / score / vis 相当がある場合は優先して移植・接続する。

## 4. 問題固有編集

編集中心は以下に限定する。

- `wasm/src/impl_vis.rs`
  - `generate(seed) -> String`
  - `calc_max_turn(input, output) -> usize`
  - `visualize(input, output, turn) -> Result<(i64, String, String), String>`
- `vite.config.js`
  - `parseCaseMeta(filePath)` に case metadata 抽出を追加する
  - `CASE_SORT_OPTIONS` に必要な sort option を追加する
- `src_eval/case_sorters.js`
  - metadata sort に対応する sorter を追加する

問題固有 metadata は `results/eval_records.jsonl` に混ぜない。eval viewer 側で `input_dir` と `case_name` から case file を読んで復元する。

## 5. フロントエンド接続

- `wasm/Cargo.toml` の crate 名と `src_vis/main.js` の WASM import 名を合わせる
- `/` は case visualizer、`/eval.html` は eval viewer として維持する
- UI の大枠は原則変更しない

## 6. ビルドと確認

以下を通す。

```sh
cd wasm && cargo check
cd wasm && wasm-pack build --target web --out-dir ../src_vis/wasm
corepack yarn build
./scripts/dev_vis.sh
```

`./scripts/dev_vis.sh` は起動できたことが分かったらすぐ終了する。ユーザーから明示的な指示がない限り、Playwright やブラウザ操作で `/` と `/eval.html` を確認しない。

## 7. 完了条件

- `cargo check`, `wasm-pack build`, `corepack yarn build` が通る
- `./scripts/dev_vis.sh` で起動できることを確認し、起動できたらすぐ終了する
- 最後の出力で、ユーザーが起動するためのコマンド `./scripts/dev_vis.sh` を提示する

## 禁止事項

- 同梱テンプレートを使わず UI/API を作り直すこと
- 公式 visualizer があるのに独自描画を優先すること
- `src_vis/wasm/` の生成物を手書き編集すること
- `eval_records.jsonl` に問題固有 metadata を追加すること
- この skill の同梱テンプレート自体を編集すること
