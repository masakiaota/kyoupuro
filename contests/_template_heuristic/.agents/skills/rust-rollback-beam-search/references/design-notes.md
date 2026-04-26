# 設計メモ

## 通常 beam search との違い

通常の beam search は候補ごとに `State` を clone して `Vec<State>` に持つことが多い。差分更新型では、探索木の辺を順にたどり、1つの `State` を `move_forward(action)` / `move_backward(action)` で動かす。

```text
root
├── a
│   ├── c
│   └── d
└── b
    └── e
```

たとえば `a -> c` を見た後に `a -> d` を見るなら、`c` を戻して `d` を進める。`b -> e` へ移るなら、`d` と `a` を戻し、`b` と `e` を進める。

## Rust 化の方針

- `Action` は `Copy` にする。戻すための情報が必要なら `Action` に含める。
- `State` は clone しない。clone 型 beam の代替ではなく、差分更新を前提に設計する。
- `Evaluator` は `Ord` にする。`BinaryHeap` や segment tree で扱いやすい。
- `HashKey` は `u64` を標準にする。必要なら tuple や custom key に変更する。
- `Selector` は hash ごとの最良候補だけを残し、beam 幅を超えたら最悪候補を置き換える。

## 3つのテンプレート

### Euler Tour 辺列版

推奨版。現在の beam の各 leaf へ移動するための `Forward(node)` / `Backward(node)` / `Visit(node)` の列を作り、その列に従って `State` を動かす。構造が平坦で Rust でも扱いやすい。

### 二重連鎖木版

各 node が `first_child` と `next_sibling` を持つ。DFS で子へ降りるときに `move_forward`、戻るときに `move_backward` する。元記事の発想に近いが、Rust では index ベースで持つのが扱いやすい。

### 可変ターン版

`Action::step()` が `1` とは限らない。turn `t` の beam から `t + step` の selector に候補を送る。最短手数探索、複数操作をまとめた macro action、局所探索の枝刈りなどに使う。

## 使い分け

- 固定長の解を作る、または各手が必ず1ターン進む: Euler Tour 辺列版。
- 元記事の木構造を保ったまま移植したい: 二重連鎖木版。
- action によって消費ターンが変わる: 可変ターン版。

## 設計上の注意

- `move_forward` と `move_backward` は副作用の順序まで対称にする。
- `hash_key` は `move_forward` 後に更新済みの状態から返す。
- `evaluate` は `move_forward` 後の状態で計算する。
- `Selector` の同一 hash 排除は強い仮定である。同じ hash にまとめてはいけない情報があるなら key に含める。
- `Action` が大きくなる場合、復元情報を全て `Action` に持たせる方式が重くなる。必要なら問題固有に undo stack 方式へ変える。

## 出典の扱い

eijirou さんの記事は実装方式の背景として参照する。skill 内では記事本文やコードを転載せず、Rust で使うための構造と運用だけをまとめる。

https://eijirou-kyopro.hatenablog.com/entry/2024/02/01/115639
