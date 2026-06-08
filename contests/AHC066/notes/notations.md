# 記号一覧

このファイルは、問題で使う記号と、実装・会話で使う代表名・型・制約の正本である。

## notation 方針

- 公式記号名は保持する。公式が `N`, `M`, `T` なら `N`, `M`, `T` と書き、Rust 風の `n`, `m`, `t` へ直さない。
- 添字は 0-based とし、`v[i,j]`, `h[i,j]`, `a[t]` のような配列アクセス風表記を使う。
- 座標は `(i,j)` で表し、`i` は下方向、`j` は右方向である。
- `v000_template.rs` では高速化のため、盤面座標 `(i,j)` を 1 次元の `cell = i * n + j` に潰して扱う。notation では必要に応じて `cell(i,j)` と書く。
- `v000_template.rs` では `Option<usize>` の代わりに `u8` と sentinel `NONE = 255` を使う。ボール・かご種類 `k` は `0 <= k < M <= 40` なので `u8` に収まる。
- 解法考察や仮説は混ぜない。状態量・添字・型・不変条件だけを書く。

## v000 実装定数

### 固定上限 `MAX_N`, `MAX_M`, `MAX_CELLS`

`v000_template.rs` で固定長配列を使うための上限である。

- 議論用 notation: `MAX_N`, `MAX_M`, `MAX_CELLS`
- コード上の代表名: `MAX_N`, `MAX_M`, `MAX_CELLS`
- Rust 型: `usize` の `const`
- 値・不変条件: `MAX_N = 20`, `MAX_M = 40`, `MAX_CELLS = MAX_N * MAX_N = 400`
- 備考: 問題制約 `N <= 20`, `M <= 2N <= 40` に対応する。

### 空値 sentinel `NONE`

ボール・かご・手持ちが存在しないことを表す sentinel である。

- 議論用 notation: `NONE`
- コード上の代表名: `NONE`
- Rust 型: `u8` の `const`
- 値・不変条件: `NONE = 255`; 有効な種類 `k` は `0 <= k < M <= 40` なので衝突しない
- 備考: `Option<usize>` より軽い表現として、`init_ball_at`, `basket_at`, `cell_ball`, `held` で使う。

## 入力サイズ

### 盤面サイズ `N`

`N x N` マスの盤面サイズを表す。

- 議論用 notation: `N`
- コード上の代表名: `N`, `n`
- Rust 型: `usize`
- index 範囲・不変条件: `10 <= N <= 20`; マス座標は `0 <= i < N`, `0 <= j < N`

### 種類数 `M`

ボールとかごの種類数を表す。各 `k` について種類 `k` のボールが 1 個、種類 `k` のかごが 1 個存在する。

- 議論用 notation: `M`
- コード上の代表名: `M`, `m`
- Rust 型: `usize`
- index 範囲・不変条件: `N/2 <= M <= 2N`; `0 <= k < M`

### 基本操作回数上限 `T`

マクロ展開後に実行される基本操作回数の上限を表す。`T+1` 回目の基本操作は実行されずに打ち切られる。

- 議論用 notation: `T`
- コード上の代表名: `T`, `t_limit`
- Rust 型: `usize`
- index 範囲・不変条件: `1 <= T <= 2N^2M`
- 備考: 出力操作列長 `A` も `A <= T` でなければならない。

## 盤面・壁

### 位置 `(i,j)`

盤面上のマス座標を表す。左上が `(0,0)` である。

- 議論用 notation: `(i,j)`
- コード上の代表名: `pos`, `Point { i, j }`, `(usize, usize)`
- Rust 型: `(usize, usize)` または専用 `struct`
- index 範囲・不変条件: `0 <= i < N`, `0 <= j < N`

### 1 次元マス番号 `cell`

`v000_template.rs` で高速化のために使う 1 次元化したマス番号である。

- 議論用 notation: `cell`, `cell(i,j)`
- コード上の代表名: `cell`, `pos`
- Rust 型: `usize` または保存時は `u16`
- index 範囲・不変条件: `cell = i * n + j`; `0 <= cell < n * n <= MAX_CELLS`
- 備考: `Grid::cell(i,j)` で `(i,j) -> cell`、`Grid::ij(cell)` で `cell -> (i,j)` に変換する。

### 方向 `dir`

上下左右の 4 方向を表す番号である。

- 議論用 notation: `dir`
- コード上の代表名: `dir`
- Rust 型: `usize` または `u8`
- index 範囲・不変条件: `0=上`, `1=右`, `2=下`, `3=左`
- 備考: `v000_template.rs` では右折を `(dir + 1) & 3`、左折を `(dir + 3) & 3` で処理する。

### 横方向の壁 `v[i,j]`

マス `(i,j)` と `(i,j+1)` の間の壁を表す。

- 議論用 notation: `v[i,j]`
- コード上の代表名: `v`, `vertical_walls`, `right_wall`
- Rust 型: `Vec<Vec<bool>>` または `Vec<Vec<u8>>`
- index 範囲・不変条件: `0 <= i < N`, `0 <= j < N-1`; `1` なら壁あり、`0` なら壁なし
- 備考: 入力では `v_0, ..., v_{N-1}` として、各行が長さ `N-1` の `01` 文字列で与えられる。`v000_template.rs` では読み取り後に `wall_mask[cell]` へ統合する。

### 縦方向の壁 `h[i,j]`

マス `(i,j)` と `(i+1,j)` の間の壁を表す。

- 議論用 notation: `h[i,j]`
- コード上の代表名: `h`, `horizontal_walls`, `down_wall`
- Rust 型: `Vec<Vec<bool>>` または `Vec<Vec<u8>>`
- index 範囲・不変条件: `0 <= i < N-1`, `0 <= j < N`; `1` なら壁あり、`0` なら壁なし
- 備考: 入力では `h_0, ..., h_{N-2}` として、各行が長さ `N` の `01` 文字列で与えられる。`v000_template.rs` では読み取り後に `wall_mask[cell]` へ統合する。

### 壁 bitmask `wall_mask[cell]`

`cell` から見て隣接マスとの間に壁がある方向を bitmask で表す。外周は含めない。

- 議論用 notation: `wall_mask[cell]`
- コード上の代表名: `wall_mask`
- Rust 型: `[u8; MAX_CELLS]`
- index 範囲・不変条件: `0 <= cell < n*n`; bit `1 << dir` が立っていればその方向に壁あり
- 備考: `v[i,j] = 1` なら左 cell の右 bit と右 cell の左 bit を立てる。`h[i,j] = 1` なら上 cell の下 bit と下 cell の上 bit を立てる。

### 外周 bitmask `edge_mask[cell]`

`cell` から見て盤面外へ出ずに進める方向を bitmask で表す。壁は考慮しない。

- 議論用 notation: `edge_mask[cell]`
- コード上の代表名: `edge_mask`
- Rust 型: `[u8; MAX_CELLS]`
- index 範囲・不変条件: `0 <= cell < n*n`; bit `1 << dir` が立っていれば外周には当たらない

### 移動可能 bitmask `move_mask[cell]`

`cell` から見て外周と壁の両方を考慮して進める方向を bitmask で表す。

- 議論用 notation: `move_mask[cell]`
- コード上の代表名: `move_mask`
- Rust 型: `[u8; MAX_CELLS]`
- index 範囲・不変条件: `move_mask[cell] = edge_mask[cell] & !wall_mask[cell]`

### 次マス表 `next_cell[cell,dir]`

前進 `F` をした後の `cell` を事前計算した表である。

- 議論用 notation: `next_cell[cell,dir]`
- コード上の代表名: `next_cell`
- Rust 型: `[[u16; 4]; MAX_CELLS]`
- index 範囲・不変条件: 進める場合は隣接 cell、外周または壁で進めない場合は同じ `cell`
- 備考: `State::apply_basic(OP_F)` は `grid.next(pos, dir)` を参照するだけでよい。

## ボール・かご

### ボール初期位置 `(b[k],c[k])`

種類 `k` のボールの初期位置を表す。

- 議論用 notation: `(b[k],c[k])`
- コード上の代表名: `balls`, `ball_pos`
- Rust 型: `Vec<(usize, usize)>`; `v000_template.rs` では `[u16; MAX_M]`
- index 範囲・不変条件: `0 <= k < M`; すべてのボール初期位置とかご位置は相異なる
- 備考: `v000_template.rs` の `ball_pos[k]` は座標ではなく `cell(b[k],c[k])` を保持する。

### かご位置 `(d[k],e[k])`

種類 `k` のかごの位置を表す。

- 議論用 notation: `(d[k],e[k])`
- コード上の代表名: `baskets`, `basket_pos`
- Rust 型: `Vec<(usize, usize)>`; `v000_template.rs` では `[u16; MAX_M]`
- index 範囲・不変条件: `0 <= k < M`; すべてのボール初期位置とかご位置は相異なる
- 備考: `v000_template.rs` の `basket_pos[k]` は座標ではなく `cell(d[k],e[k])` を保持する。

### 初期ボール配置 `init_ball_at[cell]`

初期状態で `cell` に置かれているボール種類を表す。問題文に公式記号はない。

- 議論用 notation: `init_ball_at[cell]`
- コード上の代表名: `init_ball_at`
- Rust 型: `[u8; MAX_CELLS]`
- index 範囲・不変条件: `0 <= cell < n*n`; ボールありなら種類 `k`、なしなら `NONE`

### かご配置 `basket_at[cell]`

`cell` にあるかごの種類を表す。問題文に公式記号はない。

- 議論用 notation: `basket_at[cell]`
- コード上の代表名: `basket_at`
- Rust 型: `[u8; MAX_CELLS]`
- index 範囲・不変条件: `0 <= cell < n*n`; かごありなら種類 `k`、なしなら `NONE`

### 盤面上のボール `cell_ball[i,j]`

シミュレーション中にマス `(i,j)` に置かれているボールの種類を表す。問題文に公式記号はない。

- 議論用 notation: `cell_ball[i,j]`
- コード上の代表名: `cell_ball`, `board_ball`
- Rust 型: `Vec<Vec<Option<usize>>>`; `v000_template.rs` では `[u8; MAX_CELLS]`
- index 範囲・不変条件: `0 <= i < N`, `0 <= j < N`; `Some(k)` または `k` は種類 `k` のボール、`None` または `NONE` はボールなし
- 備考: かごのあるマスにもボールは置かれ得る。

## ロボット状態

### ロボット位置 `robot_pos`

ロボットの現在位置を表す。初期位置は `(0,0)` である。

- 議論用 notation: `robot_pos`
- コード上の代表名: `robot_pos`, `pos`
- Rust 型: `(usize, usize)` または専用 `Point`; `v000_template.rs` では `u16` の `cell`
- index 範囲・不変条件: 常に盤面内のマス

### 向き `dir`

ロボットが向いている方向を表す。初期状態では右を向いている。

- 議論用 notation: `dir`
- コード上の代表名: `dir`, `Direction`
- Rust 型: `usize`, `i32`, `u8`, または `enum Direction`
- index 範囲・不変条件: `0=上`, `1=右`, `2=下`, `3=左`

### 手に持つボール `held`

ロボットが現在手に持っているボールの種類を表す。

- 議論用 notation: `held`
- コード上の代表名: `held`, `holding`
- Rust 型: `Option<usize>`; `v000_template.rs` では `u8`
- index 範囲・不変条件: `Some(k)` または `k` は種類 `k` のボール、`None` または `NONE` は何も持っていない

### 状態 `State`

シミュレーション中の全状態をまとめた構造体である。問題文に公式記号はない。

- 議論用 notation: `State`
- コード上の代表名: `State`
- Rust 型: `struct State`
- 主なフィールド: `pos`, `dir`, `held`, `matched`, `cell_ball`, `basic_count`, `recording`, `last_macro`, `cur_macro`
- 不変条件: `pos` は盤面内 cell、`dir < 4`、`held` と `cell_ball[cell]` は `k` または `NONE`

## 操作

### 基本操作 `F`, `R`, `L`, `S`

マクロ展開後に実行される操作である。

- `F`: 前進。壁がある場合は移動しない。
- `R`: 右折。時計回りに 90 度向きを変える。
- `L`: 左折。反時計回りに 90 度向きを変える。
- `S`: 交換。手に持っているボールと現在位置のボールを入れ替える。

`v000_template.rs` では基本操作を以下の `u8` 定数で保持する。

```text
OP_F = 0
OP_R = 1
OP_L = 2
OP_S = 3
```

### コントローラー操作 `M`, `P`

出力操作列に含められるが、基本操作ではない操作である。

- `M`: マクロ記録の開始または終了。記録終了時に記録された操作列を新しいマクロとして登録する。
- `P`: 最後に登録が完了したマクロを再生する。登録済みマクロが存在しない場合は何も起こらない。

`v000_template.rs` では出力ボタンを以下の `u8` 定数で保持する。`BTN_F`, `BTN_R`, `BTN_L`, `BTN_S` はそれぞれ `OP_F`, `OP_R`, `OP_L`, `OP_S` と同じ値である。

```text
BTN_F = 0
BTN_R = 1
BTN_L = 2
BTN_S = 3
BTN_M = 4
BTN_P = 5
```

### 出力操作 `a[t]`

出力する `t` 回目のボタン操作を表す。

- 議論用 notation: `a[t]`
- コード上の代表名: `ops`, `answer`, `buttons`
- Rust 型: `Vec<char>`, `Vec<u8>`, `String`
- index 範囲・不変条件: `0 <= t < A`; 各要素は `F`, `R`, `L`, `S`, `M`, `P` のいずれか
- 備考: `v000_template.rs` では `ButtonSeq { buttons: Vec<u8> }` として保持し、出力時に `button_to_char` で 1 行 1 操作へ変換する。

### 出力操作列長 `A`

出力した操作列の長さを表す。`M` および `P` もそれぞれ 1 回のボタン操作として数える。

- 議論用 notation: `A`
- コード上の代表名: `A`, `answer_len`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= A <= T`

## マクロ状態

### 最後に登録済みのマクロ `last_macro`

最後に登録が完了したマクロの基本操作列を表す。問題文に公式記号はない。

- 議論用 notation: `last_macro`
- コード上の代表名: `last_macro`, `macro_ops`
- Rust 型: `Vec<char>`, `Vec<u8>`
- index 範囲・不変条件: 要素は基本操作 `F`, `R`, `L`, `S` のみ
- 備考: `v000_template.rs` では要素は `OP_F`, `OP_R`, `OP_L`, `OP_S` のみ。

### 記録中のマクロ `recording_macro`

現在記録中のマクロの基本操作列を表す。問題文に公式記号はない。

- 議論用 notation: `recording_macro`
- コード上の代表名: `recording_macro`, `current_macro`, `cur_macro`
- Rust 型: `Option<Vec<char>>`, `Vec<u8>` と記録中フラグ
- index 範囲・不変条件: 要素は基本操作 `F`, `R`, `L`, `S` のみ。記録中に `P` を押した場合、再生された基本操作列が追加される。
- 備考: `v000_template.rs` では `recording: bool` と `cur_macro: Vec<u8>` で表す。

### 実行済み基本操作数 `expanded_count`

マクロ展開後に実際に実行された基本操作数を表す。問題文に公式記号はない。

- 議論用 notation: `expanded_count`
- コード上の代表名: `expanded_count`, `basic_count`
- Rust 型: `usize`
- index 範囲・不変条件: `0 <= expanded_count <= T`; `T+1` 回目の基本操作は実行されない

## スコア

### 対応するかご上のボール数 `V`

シミュレーション終了時に、対応するかごのマスに置かれているボールの個数を表す。

- 議論用 notation: `V`
- コード上の代表名: `V`, `matched_count`, `matched`
- Rust 型: `usize`; `v000_template.rs` では `u8`
- index 範囲・不変条件: `0 <= V <= M`; 種類 `k` のボールが `(d[k],e[k])` に置かれていれば 1 個として数える
- 備考: `v000_template.rs` では `S` の前後で現在 cell の正誤だけを差分更新する。`is_correct_at(input, cell, ball)` は `ball != NONE && input.basket_at[cell] == ball` を判定する。

### 絶対スコア `absolute_score`

各テストケースの絶対スコアを表す。小さいほど良い。

- 議論用 notation: `absolute_score`
- コード上の代表名: `score`, `absolute_score`
- Rust 型: `usize`, `u64`
- 定義: `V = M` の場合は `A`; `V < M` の場合は `T * (M - V)`

## 実行補助

### 出力列ラッパ `ButtonSeq`

出力ボタン列を保持し、シミュレーション・スコア計算・出力文字列化を行う構造体である。問題文に公式記号はない。

- 議論用 notation: `ButtonSeq`
- コード上の代表名: `ButtonSeq`, `answer`
- Rust 型: `struct ButtonSeq { buttons: Vec<u8> }`
- index 範囲・不変条件: `buttons` の要素は `BTN_F`, `BTN_R`, `BTN_L`, `BTN_S`, `BTN_M`, `BTN_P`
- 備考: `simulate(input, grid)` は初期 `State` に全ボタンを順に適用する。

### ローカル計測 `TraceStats`

`local` feature 有効時に、fallback 回数、処理回数、処理時間を記録する構造体である。問題文に公式記号はない。

- 議論用 notation: `TraceStats`
- コード上の代表名: `TraceStats`, `trace`
- Rust 型: `struct TraceStats`
- 主なフィールド: `fallback_count`, `counts`, `times_ms`
- 備考: `local!` は `local` feature 有効時だけ中身を実行し、`local_time!` は処理時間を `times_ms` に加算する。

### 時間管理 `TimeKeeper`

ホットループで時計確認の頻度を間引くための時間管理構造体である。問題文に公式記号はない。

- 議論用 notation: `TimeKeeper`
- コード上の代表名: `TimeKeeper`, `tk`
- Rust 型: `struct TimeKeeper`
- 主なフィールド: `start`, `time_limit_sec`, `iter`, `check_mask`, `elapsed_sec`, `progress`, `is_over`
- 不変条件: `step()` は `iter` を進め、`iter & check_mask == 0` のときだけ時計を更新する。戻り値 `false` は時間切れを表す。

## 入力生成

### 壁の本数 `W`

盤面生成時に伸ばす壁の本数を表す。

- 議論用 notation: `W`
- コード上の代表名: `W`, `wall_count`
- Rust 型: `usize`
- index 範囲・不変条件: `W = rand(0,N-1)`

### 順に訪れる最小手数 `X`

`(0,0)`, 各ボール初期位置、対応するかご位置を種類順に訪れる最小手数を表す。

- 議論用 notation: `X`
- コード上の代表名: `X`, `baseline_distance`
- Rust 型: `usize`
- index 範囲・不変条件: 上下左右の移動を 1 手とし、壁を越えない最短距離で計算する

### `T` 生成用乱数 `r`

`T` の生成に使う実数乱数を表す。

- 議論用 notation: `r`
- コード上の代表名: `r`
- Rust 型: `f64`
- index 範囲・不変条件: `r = rand_double(0,1)`
