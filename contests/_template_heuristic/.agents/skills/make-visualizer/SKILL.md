---
name: make-visualizer
description: AHCスタイルのヒューリスティックコンテスト用WASMビジュアライザを実装する。problem_description.txt と tools/src/ が揃っているときに使用する。公式 `tools/src` と整合し、最終的に build と起動まで通る状態を完成条件とする。
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
- `src_vis/wasm/` の生成物が無いときは `./scripts/dev_vis.sh` が先に `./scripts/build_wasm.sh` を呼ぶ前提である。
- 現在のテンプレートは単一問題前提である。問題切り替え UI は最初から入っていない。
- 入力操作系は左サイドバーに寄せ、可視化内容は右ゾーンに寄せる。
- 左サイドバーの操作は上から下へ 1 本の流れで並べる。

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

`tools/src/lib.rs` だけで意味論や描画ロジックの把握に不足がある場合は、`lib.rs` から参照される `tools/src` 配下の非 `bin` モジュールを追加で読むこと。
`tools/src/bin/` は原則読まない。

読んだら、実装せずに以下をユーザーに提示すること。

- 描画する要素
- ターンの定義
- ターンごとの状態変化
- スコア表示
- SVG の全体レイアウト
- 色分けや強調表示の方針
- 左サイドバーの上から順に `Rust bin`、`case`、`実行`、`Turn`、再生操作、速度、ループ、`Score`、`Error`、`Input`、`Output` を置くレイアウト
- `case` 選択の左右に `◁` / `▷` を置くこと
- 右側は `svgHost` を中心とした可視化領域にすること

この設計でよいか承認を得てから実装に進むこと。

## ステップ3: Rust 側を実装する

- `tools/src` のうち、パース・状態遷移・スコア計算・可視化に必要なロジックを `wasm/src/impl_vis.rs` から利用できる形にする。
- 実装手段は固定しない。必要な部分だけを移植してもよいし、補助モジュールを踏まえて組み直してもよい。
- 外部プロセス実行、標準入出力依存、ファイル I/O など、WASM で不要な処理は除去または回避する。
- `generate`, `calc_max_turn`, `visualize` の 3 関数はテンプレートが要求するシグネチャを維持する。
- `calc_max_turn` は、出力が非空なら 1 以上を返すこと。
- SVG 生成は Rust 側に寄せ、`visualize` は `(score, err, svg)` を返すこと。
- `tools/src/bin/` は根拠にしない。`lib.rs` だけで不十分なときは、必要な非 `bin` モジュールを読んで整合性を確保すること。

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
import init, { gen, get_max_turn, vis } from "./wasm/your_vis_crate.js";

async function main() {
  await init();
  // UI 初期化
}
```

- `public/wasm/` を import してはならない。
- `fetch("/wasm/...")` や `Blob` import を追加してはならない。
- wasm のファイル名は `wasm/Cargo.toml` の crate 名に従う。必要なら生成後に `src_vis/wasm/` を確認して import 名を合わせること。
- `await init()` で足りない場合のみ、wrapper が要求する引数に合わせて `await init(wasmUrl)` を使うこと。
- `Rust bin` と `case` が確定したら、その組に対応する既存の `in/out` をただちに読み込んで描画すること。
- `out` が存在しない場合のみ、`実行` ボタンで現在の `in` に対して solver を走らせ、その結果を `Output` に反映して即時可視化すること。
- 自動可視化時は必ず最後の turn を選ぶこと。初期 turn は終端状態と score 確認を優先する。
- `case` セレクトの右に `◁` / `▷` を置き、前後 case へ即座に移動できるようにすること。移動後は自動で最後の turn まで描画すること。
- `Rust bin` と `case` を変えた時点で、手動の読込ボタンは不要である。
- 再生操作の並び順は `再生` / `◁` / `▷` に固定すること。
- 最後の turn で `再生` を押したときは、最初の turn に戻ってから再生を開始すること。
- `loop` は既定で `off` にすること。
- 高速再生で描画が追いつかない場合に備え、描画更新はフレームレート基準で間引くこと。
- 再生は `requestAnimationFrame` ベースを推奨し、速度が高い場合は 60Hz 以下の描画更新へ落とし込むこと。
- 1 tick ごとに 1 turn 進める固定ではなく、経過時間から複数 step をまとめて消化できる実装を標準とすること。

## ステップ5: ビルド

以下を完了条件とする。

```bash
cd wasm && cargo check
```

続いて、WASM を `src_vis/wasm/` に出力する。

```bash
cd wasm && wasm-pack build --target web --out-dir ../src_vis/wasm
```

最後に、Vite 側が起動することを確認する。

```bash
./scripts/dev_vis.sh
```

## ステップ6: 動作確認

完了状態は以下の 3 つである。

1. `cargo check` が通る
2. `cd wasm && wasm-pack build --target web --out-dir ../src_vis/wasm` が通る
3. `./scripts/dev_vis.sh` で起動できる

入力生成、ターン操作、SVG 表示などの手動確認は有益だが、必須完了条件ではない。

## 禁止事項

- `public/wasm/*.js` を import すること
- `public/wasm/*.wasm` を source import すること
- `fetch + Blob + import(blobUrl)` を標準構成として採用すること
- `tools/src/bin/` を読んで設計を決めること
- `lib.rs` だけで不十分なのに、必要な非 `bin` モジュールを読まずに進めること
- 操作系を左右に分散させること
- `bin` / `case` 確定後に追加の「読込」ボタンを要求すること
- 既存の `in/out` が確定しているのに手動反映前提の UX にすること
- 高速再生でフレーム落ちを放置したまま完了扱いにすること
