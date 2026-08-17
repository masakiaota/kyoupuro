# 設計メモ

## 通常の beam search との違い

通常の beam search は候補ごとに `State` をコピーすることが多い。差分更新型では探索木の辺を順にたどり、1つの `State` を `move_forward(action)` / `move_backward(action)` で動かす。

```text
root
├── a
│   ├── c
│   └── d
└── b
    └── e
```

`a -> c` の後に `a -> d` を見るなら `c` を戻して `d` を進める。`b -> e` へ移るなら `d` と `a` を戻し、`b` と `e` を進める。

## C++ 実装の方針

- `Action` は小さな値型にする。戻すための情報が必要なら `Action` に含める。
- `State` は候補ごとにコピーせず、差分更新を前提に設計する。
- `Evaluator` は全順序を持たせる。
- `HashKey` は `uint64_t` を標準にし、必要なら問題固有の型へ変える。
- `Selector` は hash ごとの最良候補だけを残し、beam 幅を超えたら最悪候補を置き換える。
- node 間の参照はポインタではなく `vector` 上の index で保持する。

## 3つのテンプレート

### Euler Tour 辺列版

現在の beam の各 leaf へ移動する `Forward(node)` / `Backward(node)` / `Visit(node)` の列を作り、その列に従って `State` を動かす。固定ターンではこれを基本とする。

### 二重連鎖木版

各 node が `first_child` と `next_sibling` を持つ。DFS で子へ降りるときに `move_forward`、戻るときに `move_backward` する。

### 可変ターン版

`Action::step()` が1とは限らない。turn `t` の beam から `t + step` の selector に候補を送る。macro action や消費ターンの異なる操作に使う。

## 注意点

- `move_forward` と `move_backward` は副作用の順序まで対称にする。
- 同じ hash にまとめてはいけない情報があるなら、必ず key に含める。
- `Action` が大きくなりすぎる場合だけ、問題固有の undo stack を検討する。

実装方式は eijirou 氏の「差分更新ビームサーチ実装」を参考にしている。

https://eijirou-kyopro.hatenablog.com/entry/2024/02/01/115639
