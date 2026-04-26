---
name: rust-rollback-beam-search
description: Manual-reference skill for Rust rollback/delta-state beam search templates. Use only when the user explicitly asks to use rust-rollback-beam-search, rollback beam search, 差分更新型ビームサーチ, or the bundled Rust templates.
---

# rust-rollback-beam-search

Rust で差分更新型ビームサーチを書くための手動参照 skill である。通常の beam search と違い、各候補に `State` を clone して持たせず、1つの `State` を `move_forward(action)` / `move_backward(action)` で進退させながら候補を展開する。

この skill は自動利用を前提にしない。ユーザーがこの skill 名、差分更新型ビームサーチ、rollback beam search、または同梱テンプレートを明示した場合だけ参照する。

## 最初に読むもの

1. `references/template-usage.md`
2. 必要に応じて `references/design-notes.md`
3. 具体例が欲しい場合は `references/examples.md`

## テンプレート選択

- 基本は `assets/rust/euler_tour_edges_rollback_beam.rs` を使う。
- 元記事に近い二重連鎖木の構造を見たい場合は `assets/rust/linked_tree_rollback_beam.rs` を使う。
- 1つの `Action` が複数ターン進む問題では `assets/rust/variable_turn_rollback_beam.rs` を使う。

## 実装時の原則

- `Action` は小さく `Copy` できる型にする。`move_backward(action)` に必要な情報も `Action` に含める。
- `Evaluator` は `Ord + Copy` にし、小さいほど良い評価に揃える。最大化問題では符号反転する。
- `HashKey` は同一視したい状態を表す。既定テンプレートは `u64` を使う。
- `State` は clone しない。大きい盤面、累積スコア、差分更新用の補助情報をここへ持たせる。
- `move_forward` と `move_backward` が完全に逆操作になることを、最初に小さい入力で検証する。
- 問題固有に書き換える箇所は `TODO(problem)` を検索して埋める。

## 出典

設計の背景は eijirou さんの「差分更新ビームサーチ実装」記事を参考にする。

https://eijirou-kyopro.hatenablog.com/entry/2024/02/01/115639
