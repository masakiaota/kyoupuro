# テンプレートの使い方

## 1. どの `.rs` を使うか決める

- `euler_tour_edges_rollback_beam.rs`: 基本推奨。固定ターンで1手ずつ進む AHC 形式に向く。
- `linked_tree_rollback_beam.rs`: 元記事の二重連鎖木版に近い構造を確認したいときに使う。
- `variable_turn_rollback_beam.rs`: 1つの `Action` が `1..=k` ターン進む問題に使う。

迷ったら `euler_tour_edges_rollback_beam.rs` から始める。

## 2. solver にコピーする

例:

```sh
SKILL_DIR=".agents/skills/rust-rollback-beam-search"
cp "$SKILL_DIR/assets/rust/euler_tour_edges_rollback_beam.rs" src/bin/v001_rollback_beam.rs
```

既存 solver に組み込む場合は、必要な型と関数だけを貼り込む。

## 3. `TODO(problem)` を埋める

最低限、以下を問題固有に置き換える。

- `Action`: 操作、移動、配置、削除などの1手を表す。
- `State`: 盤面、位置、使用済み資源、累積スコア、hash 更新用の値を持つ。
- `State::enumerate_actions`: 現在状態から出せる候補手を列挙する。
- `State::move_forward`: `Action` を適用して状態を進める。
- `State::move_backward`: 同じ `Action` を使って状態を完全に戻す。
- `State::evaluate`: beam 内の比較値を返す。小さいほど良い。
- `State::hash_key`: 重複排除したい粒度の hash を返す。

## 4. 評価値と hash を決める

`Evaluator` は小さいほど良い値に統一する。最大化したいスコア `score` があるなら、基本は `score_key = -score` にする。

`HashKey` は「同じなら片方だけ残してよい」状態を表す。強くしすぎると多様性が消え、弱くしすぎると同じ状態が大量に残る。

## 5. 逆操作を先に検証する

beam 幅や評価関数を調整する前に、次を確認する。

```rust
let before = state.debug_snapshot();
state.move_forward(action);
state.move_backward(action);
assert_eq!(before, state.debug_snapshot());
```

テンプレートには汎用の `debug_snapshot` は入れていない。問題ごとに、盤面、位置、スコア、hash など比較したい値を tuple や軽量 struct で返す関数を一時的に追加する。

## 6. AHC での運用

- `beam_width` は最初は小さくし、正しく動くことを確認してから広げる。
- `enumerate_actions` は候補数を絞る。差分更新が速くても、候補数が大きすぎるとすぐ詰まる。
- `move_backward` の漏れはスコア劣化ではなく破壊的なバグになる。assert を厚くしてよい。
- 1会話で新規 solver 候補を複数生成して自動比較しない。既存候補の比較はユーザーが明示した場合だけ行う。
