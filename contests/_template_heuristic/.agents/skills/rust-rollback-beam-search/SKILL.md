---
name: rust-rollback-beam-search
description: Rust rollback/delta-state beam search templates for AHC-style solvers. Provides bundled Rust templates for Euler Tour edge traversal, linked-tree traversal, and variable-turn rollback beam search.
---

# rust-rollback-beam-search

Rust で差分更新型ビームサーチを書くためのテンプレート集である。通常の beam search と違い、候補ごとに `State` を clone せず、1つの `State` を `move_forward(action)` / `move_backward(action)` で進退させながら候補を展開する。


## テンプレート選択

- 基本は `assets/rust/euler_tour_edges_rollback_beam.rs` を使う。固定ターンで1手ずつ進む AHC 形式に向く。
- 元記事に近い二重連鎖木版を見たい場合は `assets/rust/linked_tree_rollback_beam.rs` を使う。
- 1つの `Action` が複数ターン進む問題では `assets/rust/variable_turn_rollback_beam.rs` を使う。

迷ったら Euler Tour 辺列版から始める。

## 導入手順

solver にテンプレートをコピーする。

例:
```sh
SKILL_DIR=".agents/skills/rust-rollback-beam-search"
solver_name="v001_rollback_beam"
awk -v file_name="${solver_name}.rs" '
  NR == 1 { print "// " file_name }
  { print }
' "$SKILL_DIR/assets/rust/euler_tour_edges_rollback_beam.rs" > "src/bin/${solver_name}.rs"
```

このコマンドは、`src/bin` の規約どおり 1 行目にファイル名コメントを置き、続く `#![allow(dead_code, non_snake_case)]` を crate 属性として有効にする。`non_snake_case` により、問題文の `N`、`M` などをそのまま使える。

既存 solver に組み込む場合は、必要な型と関数だけを貼り込む。属性は途中に貼れないため、対象 solver の先頭に `#![allow(non_snake_case)]` があることを確認する。

## 埋める箇所

`TODO(problem)` を検索し、最低限以下を問題固有に置き換える。

- `Action`: 操作、移動、配置、削除などの1手を表す。小さく `Copy` できる型にする。
- `State`: 盤面、位置、使用済み資源、累積スコア、hash 更新用の値を持つ。
- `State::enumerate_actions`: 現在状態から出せる候補手を列挙する。
- `State::move_forward`: `Action` を適用して状態を進める。
- `State::move_backward`: 同じ `Action` を使って状態を完全に戻す。
- `State::evaluate`: beam 内の比較値を返す。小さいほど良い。
- `State::hash_key`: 重複排除したい粒度の hash を返す。

## 実装時の原則

- `move_backward(action)` に必要な情報は `Action` に含める。
- `Evaluator` は `Ord + Copy` にし、小さいほど良い評価に揃える。最大化問題では符号反転する。
- `HashKey` は同一視したい状態を表す。既定テンプレートは `u64` を使う。
- `State` は clone しない。大きい盤面、累積スコア、差分更新用の補助情報をここへ持たせる。
- `hash_key` と `evaluate` は `move_forward` 後の状態で計算する。
- 可変ターン版では `Action::step()` を必ず `1` 以上にする。

最大化したいスコア `score` があるなら、基本は `Evaluator { score_key: -score, ... }` のように符号反転する。

## 逆操作チェック

beam 幅や評価関数を調整する前に、`move_forward` と `move_backward` が完全に逆操作になることを確認する。

```rust
let before = state.debug_snapshot();
state.move_forward(action);
state.move_backward(action);
assert_eq!(before, state.debug_snapshot());
```

テンプレートには汎用の `debug_snapshot` は入れていない。問題ごとに、盤面、位置、スコア、hash など比較したい値を tuple や軽量 struct で返す関数を一時的に追加する。

## AHC での運用

- `beam_width` は最初は小さくし、正しく動くことを確認してから広げる。
- `enumerate_actions` は候補数を絞る。差分更新が速くても、候補数が大きすぎると詰まる。
- `move_backward` の漏れはスコア劣化ではなく破壊的なバグになる。assert を厚くしてよい。

## その他資料
- `references/design-notes.md`: 通常 beam search との違い、3つのテンプレートの構造、Rust 化方針。
- `references/examples.md`: `Action`、`Evaluator`、`hash_key`、逆操作チェックの具体例
- このスキルは eijirou さんの「差分更新ビームサーチ実装」記事を参考に作成された。元記事を Rust 向けに書き直したものである。https://eijirou-kyopro.hatenablog.com/entry/2024/02/01/115639
