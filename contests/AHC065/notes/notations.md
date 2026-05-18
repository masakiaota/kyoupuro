# 記号一覧

このファイルは、問題で使う記号と、実装・会話で使う代表名・型・制約の正本である。

## notation 方針

- 公式記号名は保持する。`N`, `M`, `T`, `B` は Rust 風の `n`, `m`, `t`, `b` へ直さず議論する。
- 添字は 0-based とし、配列は `a[i,j]`, `C[m][x]` のように書く。
- 座標 `(i,j)` は `i` が行、`j` が列である。下方向が `i+1`、右方向が `j+1`。
- 解法考察や仮説は混ぜない。状態量・添字・型・不変条件だけを書く。

## 固定サイズ・座標

### 盤面サイズ `N`

- 議論用 notation: `N`
- コード上の代表名: `N`
- Rust 型: `usize`
- index 範囲・不変条件: すべてのテストケースで `N = 20`。盤面は `N x N`、マス数は `N^2 = 400`。

### 座標 `(i,j)`

- 議論用 notation: `(i,j)`
- コード上の代表名: `(i, j)`, `Pos`, `Point`
- Rust 型: `(usize, usize)` または専用 `struct`
- index 範囲・不変条件: `0 <= i < N`, `0 <= j < N`。一番左上が `(0,0)`。

### 搬出口 `E`

- 議論用 notation: `E = (0,N/2)`
- コード上の代表名: `exit`, `EXIT`, `EXIT_POS`
- Rust 型: `(usize, usize)`
- index 範囲・不変条件: `N = 20` なので `E = (0,10)`。

## 入力

### 初期配置 `a[i,j]`

- 議論用 notation: `a[i,j]`
- コード上の代表名: `a`, `grid`, `initial`
- Rust 型: `Vec<Vec<usize>>` または `[[usize; 20]; 20]`
- index 範囲・不変条件: `0 <= i,j < N`。値は `0 <= a[i,j] < N^2`。
- 備考: `a` には `0..N^2` の整数がそれぞれちょうど 1 回ずつ現れる。

### 箱番号 `k`

- 議論用 notation: `k`
- コード上の代表名: `box_id`, `k`, `target`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= k < N^2`。箱は番号の小さい順に搬出される。

## ベルトコンベア

### ベルトコンベア個数 `M`

- 議論用 notation: `M`
- コード上の代表名: `M`, `conveyors.len()`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= M <= N^2`。

### `m` 番目のベルトコンベア `C[m]`

- 議論用 notation: `C[m] = [(i[m,0],j[m,0]), ..., (i[m,l_m-1],j[m,l_m-1])]`
- コード上の代表名: `conveyors[m]`, `loops[m]`
- Rust 型: `Vec<Vec<(usize, usize)>>`
- index 範囲・不変条件: `0 <= m < M`。同一 `C[m]` 内のマスはすべて相異なる。

### ベルトコンベア長 `l_m`

- 議論用 notation: `l_m`
- コード上の代表名: `l`, `len`, `conveyors[m].len()`
- Rust 型: `usize`
- index 範囲・不変条件: `l_m >= 2`。

### ベルトコンベア上の添字 `x`

- 議論用 notation: `x`
- コード上の代表名: `x`, `idx`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= x < l_m`。
- 備考: 隣り合う `x` と `x+1`、および `l_m-1` と `0` のマスは上下左右に隣接する。

### マスの被覆数 `cover[i,j]`

- 議論用 notation: `cover[i,j]`
- コード上の代表名: `cover`, `cell_loop_count`
- Rust 型: `Vec<Vec<usize>>` または `[[usize; 20]; 20]`
- index 範囲・不変条件: すべてのマスで `0 <= cover[i,j] <= 2`。
- 備考: 問題文では明示配列として与えられないが、出力検証で使う代表状態量である。

## 操作列

### 操作回数 `T`

- 議論用 notation: `T`
- コード上の代表名: `T`, `ops.len()`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= T <= 100000`。

### `t` 回目の操作 `(m_t,d_t)`

- 議論用 notation: `(m_t,d_t)`
- コード上の代表名: `ops[t]`, `Operation { m, d }`
- Rust 型: `(usize, i32)` または専用 `struct`
- index 範囲・不変条件: `0 <= t < T`, `0 <= m_t < M`, `d_t in {-1, 1}`。

### 循環移動

- 議論用 notation: `x -> (x + d_t) mod l_m`
- コード上の代表名: `next_idx`, `rotate`
- Rust 型: `usize`
- index 範囲・不変条件: 操作対象は `C[m_t]` 上の箱または空きマスである。
- 備考: Rust で `d_t = -1` を扱うときは、負の剰余を避ける実装にする。

## 搬出・得点

### 搬出済み個数 `B`

- 議論用 notation: `B`
- コード上の代表名: `B`, `removed`, `delivered`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= B <= N^2`。
- 備考: `B` 個搬出済みのとき、次に搬出されるべき箱番号は `B` である。

### 空きマス

- 議論用 notation: `empty`
- コード上の代表名: `empty`, `None`
- Rust 型: `Option<usize>` を使う場合は `None`
- index 範囲・不変条件: 搬出済みの箱があった位置は空きマスとなり、ベルトコンベア操作では箱と同様に循環移動する。

### 得点 `score`

- 議論用 notation: `score`
- コード上の代表名: `score`
- Rust 型: `i64`
- index 範囲・不変条件:
  - `B = N^2` の場合、`10^6 + round(10^6 log_2(10^5 / T))`
  - `B < N^2` の場合、`round(10^6 * B / N^2)`
