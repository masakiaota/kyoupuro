# masters2026-qual Visualizer (Development Share)

このディレクトリには、`masters2026-qual` 向けの可視化ツール一式が入っている。

- フロントエンド: Vite (`index.html`, `src_vis/main.js`)
- WASMロジック: Rust (`wasm/src/lib.rs`, `wasm/src/impl_vis.rs`)
- 生成済みWASM: `src_vis/wasm/*`
- 入力・採点用ツール: `tools/`

## 1. 前提環境

最低限必要なもの:

- Node.js 18 以上
- Corepack (`corepack` コマンド)

WASM側を編集して再ビルドする場合に必要:

- Rust toolchain (推奨: 1.89.0)
- `wasm-pack`

`wasm-pack` が無い場合:

```bash
cargo install wasm-pack
```

## 2. セットアップ

```bash
corepack yarn install
```

## 3. 開発サーバー起動

```bash
corepack yarn dev
```

起動後にブラウザで以下を開く:

- <http://127.0.0.1:5173/>

## 4. 基本操作

- `生成`: seed/problem(A,B,C) から入力を生成
- `サンプル`: `samples/input_01.txt`, `samples/output_01.txt` を読み込み
- `Rust bin`: `src/bin/*.rs` の一覧取得と実行
  - `bin更新`: 一覧を再取得
  - `bin実行→Output反映`: 選択したbinを入力テキストで実行し、stdoutをOutput欄へ反映
- `View`:
  - `1: 担当領域` 各マスの主担当ロボットを表示（重複は斜線）
  - `2: 重複度` 各マスを被覆ロボット数のヒートマップで表示
  - `3: ロボット単体` 選択ロボットの経路（head/tail）と担当セルを強調表示
- Output欄に解答を貼るとターン上限更新
- スライダーと再生UIでターン表示を確認

再生UI:

- `◀ / ▶`: 1ターン移動
- `再生 / 停止`: 自動再生
- `速度`: 再生速度変更
- `ループ`: 最終ターン到達時に先頭へ戻すか
- キーボード: `Space`(再生/停止), `←`/`→`(1ターン移動)

## 5. WASMを再ビルドする場合（Rustを編集したとき）

`wasm/src/impl_vis.rs` などを編集したら実行する:

```bash
cd wasm
cargo check
wasm-pack build --target web --out-dir ../src_vis/wasm
cd ..
```

その後 `corepack yarn dev` を再起動する。

## 6. 本番ビルド

```bash
corepack yarn build
```

成果物は `dist/` に出る。

## 7. トラブルシュート

### `Failed to load url /wasm/...` が出る

現在は `src_vis/wasm/` を相対importする構成になっているため、
`/public` 配下を import しないこと。

### 画面が古いまま

`yarn dev` を再起動し、ブラウザをハードリロードする。

### bin一覧取得/実行に失敗する

`Rust bin` の機能は `yarn dev` 起動時のローカルAPI (`/api/*`) を使う。
静的配信 (`dist`) では使えないため、開発モードで使うこと。

## 8. 含まれている主要ファイル

- `index.html`
- `package.json`, `yarn.lock`
- `src_vis/`
- `wasm/`
- `samples/`
- `tools/`
- `problem_description.txt`
