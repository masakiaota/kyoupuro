---
name: make-visualizer
description: AHCスタイルのヒューリスティックコンテスト用WASMビジュアライザを実装する。problem_description.txt と tools/src/ が揃っているときに使用する。Vite を使う場合、WASM 生成物は `src_vis/wasm/` に置き、`src_vis/main.js` から相対 import する。
---

# make-visualizer

ヒューリスティックコンテスト用ビジュアライザを実装する。
以下の手順を順番に実行すること。

## 設計原則

- visualizer のフロントエンドが Vite のとき、wasm-pack の生成物は `src_vis/wasm/` に置く。
- `src_vis/main.js` から `./wasm/<crate_name>.js` を通常の ES module として import する。
- `public/` は固定名の静的ファイル置き場であり、WASM wrapper を import する場所ではない。
- `public/wasm/*.js` を source code から import してはならない。
- `fetch("/wasm/...js")` + `Blob` + `import(blobUrl)` は暫定回避策であり、標準構成として採用しない。
- 生成物を Vite の module graph に素直に乗せることを優先する。
- 現在のテンプレートは単一問題前提である。問題切り替え UI は最初から入っていない。

## ステップ1: 前提条件チェック

以下を確認し、問題があれば停止してユーザーに伝えること。

- `problem_description.txt` がプレースホルダーのままではないこと
- `tools/src/` が存在すること
- `src_vis/main.js` が存在すること
- `wasm/src/lib.rs` と `wasm/src/impl_vis.rs` が存在すること

## ステップ2: 問題を読んでビジュアライザ設計を提案する

以下を読むこと。

1. `problem_description.txt`
2. `tools/src/lib.rs`

読んだら、実装せずに以下をユーザーに提示すること。

- 描画する要素
- ターンの定義
- ターンごとの状態変化
- スコア表示
- SVG の全体レイアウト
- 色分けや強調表示の方針

この設計でよいか承認を得てから実装に進むこと。

## ステップ3: Rust 側を実装する

- `tools/src/lib.rs` のうち、パース・状態遷移・スコア計算に必要な部分を `wasm/src/impl_vis.rs` で利用できるようにする。
- 外部プロセス実行、標準入出力依存、ファイル I/O など、WASM で不要な処理は削除する。
- `generate`, `calc_max_turn`, `visualize` の 3 関数はテンプレートが要求するシグネチャを維持する。
- `calc_max_turn` は、出力が非空なら 1 以上を返すこと。
- SVG 生成は Rust 側に寄せ、`visualize` は `(score, err, svg)` を返すこと。
- `tools/src/bin/` は読まない。設計と実装の根拠は `problem_description.txt` と `tools/src/lib.rs` に限定すること。

テンプレートが要求するシグネチャは以下である。

```rust
pub fn generate(seed: i32) -> String
pub fn calc_max_turn(input: &str, output: &str) -> usize
pub fn visualize(input: &str, output: &str, turn: usize) -> Result<(i64, String, String), String>
```

## ステップ4: フロントエンドとの接続

- wasm-pack の生成物は `src_vis/wasm/` に出力すること。
- `src_vis/main.js` では、生成された wrapper を相対 import すること。
- 例:

```js
import init, { get_max_turn, vis, gen } from "./wasm/your_vis_crate.js";

async function main() {
  await init();
  // UI 初期化
}
```

- `public/wasm/` を import してはならない。
- `fetch("/wasm/...")` や `Blob` import を追加してはならない。
- wasm のファイル名は `wasm/Cargo.toml` の crate 名に従う。必要なら生成後に `src_vis/wasm/` を確認して import 名を合わせること。
- `await init()` で足りない場合のみ、wrapper が要求する引数に合わせて `await init(wasmUrl)` を使うこと。

## ステップ5: ビルド

まず Rust 側を確認する。

```bash
cd wasm && cargo check
```

その後、WASM を `src_vis/wasm/` に出力する。

```bash
cd wasm && wasm-pack build --target web --out-dir ../src_vis/wasm
```

Vite 側は通常通り起動する。

```bash
./scripts/dev_vis.sh
```

## ステップ6: 動作確認

以下を確認する。

1. 入力生成または入力貼り付けができる
2. 出力貼り付けでターン上限が更新される
3. スライダー操作で SVG が切り替わる
4. スコアとエラー表示が期待通り更新される

## 禁止事項

- `public/wasm/*.js` を import すること
- `public/wasm/*.wasm` を source import すること
- `fetch + Blob + import(blobUrl)` を標準構成として採用すること
- 問題切り替え UI を勝手に追加すること
- `tools/src/bin/` を読んで設計を決めること
