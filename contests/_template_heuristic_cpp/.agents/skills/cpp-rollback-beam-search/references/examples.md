# 具体例

## 固定ターン型

毎ターン1つの操作を出す問題では `euler_tour_edges_rollback_beam.cpp` を使う。

```cpp
struct Action {
    int pos;
    uint8_t old_value;
    uint8_t new_value;
    int64_t score_delta;
    uint64_t hash_delta;
};
```

`move_forward` では `grid[pos] = new_value`、`score += score_delta`、`hash ^= hash_delta` を行う。`move_backward` では逆順で戻す。

## 最大化スコア

テンプレートは小さい `Evaluator` を良い候補として扱う。

```cpp
return Evaluator{-score, static_cast<uint32_t>(turn)};
```

同点時に短い解を優先するなら `tie_break` に手数を入れる。

## hash の粒度

位置だけ同じなら同一視してよい場合は位置の hash だけを返す。使用済み資源も重要なら、回転などで混ぜて返す。

```cpp
HashKey hash_key() const {
    return position_hash ^ rotl(resource_hash, 17);
}
```

## 可変ターン型

macro action が複数ターン進む場合は `variable_turn_rollback_beam.cpp` を使う。`Action::step()` は必ず1以上にする。0を許すと同じ turn に戻る遷移になり、探索順序が壊れる。

## 逆操作チェック

一時的に snapshot を作り、全候補 Action で戻せるか確認する。

```cpp
for (const Action action : state.enumerate_actions(0)) {
    const auto before = state.debug_snapshot();
    state.move_forward(action);
    state.move_backward(action);
    assert(before == state.debug_snapshot());
}
```

snapshot はデバッグ専用なので、本番で重くてもよい。
