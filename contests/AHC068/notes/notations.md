# 記号一覧

このファイルは、問題で使う記号と、実装・会話で使う代表名・型・制約の正本である。添字はすべて 0-based とする。

## 盤面と座標

### 盤面サイズ `N`

- 議論用 notation: `N`
- コード上の代表名: `N`
- Rust 型: `usize`
- 値・不変条件: 全テストケースで `N = 20`。

### マス `(i,j)`

- 議論用 notation: `(i,j)`
- コード上の代表名: `(i, j)`、必要に応じて `Pos { i, j }`
- Rust 型: `(usize, usize)` または専用 `struct`
- index 範囲・不変条件: `0 <= i < N`、`0 <= j < N`。`i` は上から下への行、`j` は左から右への列を表す。

### カード配置 `a[i,j]`

- 議論用 notation: `a[i,j]`
- コード上の代表名: `board[i][j]`、`a[i][j]`
- Rust 型: `usize`
- index 範囲・不変条件: `a[i,j]` 全体は `0..N^2` の順列。目標位置 `(i,j)` のカード番号は `i * N + j`。

## 壁

### 縦方向の壁情報 `V[i]`

- 議論用 notation: `V[i][j]`
- コード上の代表名: `vertical_walls[i][j]`、`v[i][j]`
- Rust 型: `bool`（`true` が壁あり）または `u8`
- index 範囲・不変条件: `0 <= i < N`、`0 <= j < N-1`。マス `(i,j)` と `(i,j+1)` の間の壁を表す。

### 横方向の壁情報 `H[i]`

- 議論用 notation: `H[i][j]`
- コード上の代表名: `horizontal_walls[i][j]`、`h[i][j]`
- Rust 型: `bool`（`true` が壁あり）または `u8`
- index 範囲・不変条件: `0 <= i < N-1`、`0 <= j < N`。マス `(i,j)` と `(i+1,j)` の間の壁を表す。

### 通行可能性

- 議論用 notation: 隣接する 2 マス間に壁がないこと。
- コード上の代表名: `can_move`、`has_wall`
- 不変条件: 壁のない上下左右移動で、すべてのマス間が相互到達可能。操作によって壁配置は変化しない。

## 操作

### 長方形 `R = (r,c,h,w)`

- 議論用 notation: `R = (r,c,h,w)`
- コード上の代表名: `Rect { r, c, h, w }`
- Rust 型: `usize` フィールド 4 個の `struct` またはタプル
- index 範囲・不変条件: `h > 0`、`w > 0`、`r + h <= N`、`c + w <= N`。含まれるマスは `{ (r+x,c+y) | 0 <= x < h, 0 <= y < w }` であり、内部で隣接する任意の 2 マス間に壁がない。

### 向き `d[t]`

- 議論用 notation: `d[t]`
- コード上の代表名: `Direction::Vertical`、`Direction::Horizontal`
- Rust 型: `enum` または `char`
- 値・不変条件: 縦は `V` で、高さ `h` が偶数。横は `H` で、幅 `w` が偶数。

### t 回目の操作 `R[t]`

- 議論用 notation: `R[t] = (r[t],c[t],h[t],w[t])`
- コード上の代表名: `operations[t]`
- Rust 型: `Vec<Operation>`
- index 範囲・不変条件: `0 <= t < T`、`0 <= T <= 10^5`。縦操作は長方形の上下半分を、横操作は左右半分を、対応する各マスごとに入れ替える。

## 評価

### 操作回数 `T`

- 議論用 notation: `T`
- コード上の代表名: `operations.len()`、`T`
- Rust 型: `usize`
- 値・不変条件: `0 <= T <= 10^5`。

### 未整列マス数 `E`

- 議論用 notation: `E`
- コード上の代表名: `misplaced_count`
- Rust 型: `usize`
- 値・不変条件: 全操作後に `a[i,j] != i * N + j` であるマスの個数。

### スコア `score`

- 議論用 notation: `score`
- コード上の代表名: `score`
- Rust 型: 整数値（公式 scorer の実装に従う）
- 定義: `E = 0` の場合は `N^2 + round(10^6 * log_2(10^5 / T))`、`E > 0` の場合は `N^2 - E`。
