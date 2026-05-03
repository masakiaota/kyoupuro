# 具体例

## 固定ターン型

毎ターン1つの操作を出す問題では、`euler_tour_edges_rollback_beam.rs` を使う。

```rust
#[derive(Clone, Copy)]
struct Action {
    pos: usize,
    old_value: u8,
    new_value: u8,
    score_delta: i64,
    hash_delta: u64,
}
```

`move_forward` では `grid[pos] = new_value`、`score += score_delta`、`hash ^= hash_delta` を行う。`move_backward` では逆順で戻す。

## 最大化スコア

テンプレートは小さい `Evaluator` を良い候補として扱う。

```rust
Evaluator {
    score_key: -score,
    tie_break: turn as u32,
}
```

同点時に短い解を優先したいなら `tie_break` に手数を入れる。多様性を優先したいなら `hash_key` 側の粒度を調整する。

## hash の粒度

位置だけ同じなら同一視してよい問題:

```rust
fn hash_key(&self) -> u64 {
    self.position_hash
}
```

位置と使用済み資源の両方が重要な問題:

```rust
fn hash_key(&self) -> u64 {
    self.position_hash ^ self.resource_hash.rotate_left(17)
}
```

## 可変ターン型

macro action が複数ターン進む場合、`variable_turn_rollback_beam.rs` を使う。

```rust
#[derive(Clone, Copy)]
struct Action {
    kind: u8,
    step: usize,
    score_delta: i64,
    hash_delta: u64,
}
```

`Action::step()` は必ず `1` 以上にする。`0` を許すと同じ turn に戻る遷移になり、探索順序が壊れる。

## 逆操作チェック

一時的に snapshot を作り、全候補 action で戻せるか確認する。

```rust
let actions = state.enumerate_actions(0);
for action in actions {
    let before = (state.score, state.hash_key());
    state.move_forward(action);
    state.move_backward(action);
    assert_eq!(before, (state.score, state.hash_key()));
}
```

本番では snapshot が重くてもよい。これはデバッグ専用である。
