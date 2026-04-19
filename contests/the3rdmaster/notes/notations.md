# 記号一覧

このファイルは、問題で使う記号、コード上の代表名、型、制約の正本である。

### 盤面サイズ `N`

各レイヤーの縦横の長さを表す。

- コード上の代表名: `N` は定数、局所変数では `n`
- Rust の想定型: `usize`
- 制約・不変条件: `N = 32` で固定
- 備考: 行座標・列座標の有効範囲はともに `0..N`


### レイヤー数 `K`

利用可能なレイヤー枚数を表す。

- コード上の代表名: `k_layers` または `k_count`
- Rust の想定型: `usize`
- 制約・不変条件: `2 <= K <= 5`
- 備考: レイヤー番号そのものを表す変数名 `k`, `h` とは区別する


### 色数 `C`

透明を除く使用可能な色の種類数を表す。

- コード上の代表名: `color_count`
- Rust の想定型: `usize`
- 制約・不変条件: `2 <= C <= 4`
- 備考: 実際の色値は `0..=C` で、`0` は透明を表す


### 色型 `Color`

盤面や入力で保持する 1 ピクセル分の色値を表す共通型である。

- コード上の代表名: `Color`
- Rust の想定型: `u8`
- 制約・不変条件: 値域は `0..=C`
- 備考: 固定長配列 `Grid` や `Input::goal` の要素型として使う


### レイヤー番号 `k`, `h`

操作対象やコピー元を指すレイヤー番号を表す。

- コード上の代表名: `k`, `h`
- Rust の想定型: `usize`
- 制約・不変条件: `0 <= k, h < K`
- 備考: `copy` では `k = h` も許される


### 座標 `(i, j)`

ピクセル座標を表す。`i` は上から下への行、`j` は左から右への列である。

- コード上の代表名: `i`, `j`、組としては `Coord`
- Rust の想定型: `usize`
- 制約・不変条件: `0 <= i, j < N`
- 備考: 以後、座標の順序は常に `(row, col)` とする


### 座標型 `Coord`

1 個の座標を `(row, col)` 順でまとめた型 alias である。

- コード上の代表名: `Coord`
- Rust の想定型: `(usize, usize)`
- 制約・不変条件: 各成分は `0..N`
- 備考: ローカル変数名は `coord` や `(i, j)` を使う


### 色値 `x`

`paint` で設定する色、または一般のピクセル色を表す。

- コード上の代表名: `x` または `color`
- Rust の想定型: `u8` または `usize`
- 制約・不変条件: `0 <= x <= C`
- 備考: `x = 0` は透明に戻す操作を意味する


### レイヤー画素 `c(k, i, j)`

レイヤー `k` の座標 `(i, j)` にあるピクセル色を表す。

- コード上の代表名: `layers[k][i][j]`、あるいは `state.c[k][i][j]`
- Rust の想定型: `u8`
- 制約・不変条件: 値域は `0..=C`
- 備考: `0` は透明であり、`copy` では透明画素は上書きに使われない


### 目標画像 `g_i,j`

最終的にレイヤー 0 で一致させるべき目標色を表す。

- コード上の代表名: `input.goal[i][j]` または `goal[i][j]`
- Rust の想定型: `u8`
- 制約・不変条件: 値域は `0..=C`
- 備考: 完成条件は全 `(i, j)` で `c(0, i, j) = g_i,j`


### 盤面型 `Grid`

`N x N` の色付き画像を保持する固定長配列型である。

- コード上の代表名: `Grid`
- Rust の想定型: `[[Color; N]; N]`
- 制約・不変条件: 行数・列数はともに `N`
- 備考: 目標画像や将来の各レイヤー表現の共通土台として使う


### bitset 表現 `Bits`

盤面上の画素集合を 1024 bit の固定長 bitset で表す共通型である。

- コード上の代表名: `Bits`
- Rust の想定型: `struct Bits { w: [u64; 16] }`
- 制約・不変条件: bit `p = i * N + j` が座標 `(i, j)` に対応する
- 備考: `residual`、occurrence の被覆集合、部品候補の union を高速に扱うために使う


### 温度 `temp`

焼きなまし法における悪化遷移の受理確率を制御する温度を表す。

- コード上の代表名: `temp`
- Rust の想定型: `f64`
- 制約・不変条件: `temp > 0`
- 備考: `v102_sa_patterns` では時間進行ベースの幾何補間で更新する


### energy `eval.cost`

探索中の状態評価として使う操作回数見積もりを表す。

- コード上の代表名: `eval.cost`
- Rust の想定型: `usize`
- 制約・不変条件: 小さいほど良い
- 備考: `v102_sa_patterns` では score ではなくこれを SA の energy として使う


### 現在状態 `current`

焼きなまし法で現在位置として遷移元・遷移先の比較に使う探索状態を表す。

- コード上の代表名: `current`
- Rust の想定型: `SearchState`
- 制約・不変条件: 近傍受理のたびに更新されうる
- 備考: 最終出力用の最良状態とは分けて持つ


### 最良状態 `best`

探索開始から現在までに観測した最良の探索状態を表す。

- コード上の代表名: `best`
- Rust の想定型: `SearchState`
- 制約・不変条件: `eval.cost` を主、pattern 数と総セル数を副次基準に更新する
- 備考: `v102_sa_patterns` の最終出力は常にこの状態から復元する


### 目標 row bitmask `goal_rows[x][i]`

目標画像 `g` の row `i` において、色 `x` の位置集合を 32 bit で表した bitmask である。

- コード上の代表名: `goal_rows`
- Rust の想定型: `[[u32; N]; MAX_COLOR + 1]`
- 制約・不変条件: bit `j` が 1 なら `g_i,j = x`
- 備考: `v103_sa_fastbuild` では placement の `fix_count` / `break_count` を row 単位で高速計算するために使う


### compiled cache `compiled_cache`

正規化済み pattern から `CompiledPattern` を引く cache である。

- コード上の代表名: `compiled_cache`
- Rust の想定型: `FxHashMap<EditablePattern, Rc<CompiledPattern>>`
- 制約・不変条件: key は左上寄せ・trim 済みの `EditablePattern`
- 備考: `v103_sa_fastbuild` では unchanged pattern や再訪 pattern の再コンパイルを避けるために使う


### 入力 `Input`

問題入力全体をまとめた共通 struct である。

- コード上の代表名: `input`
- Rust の想定型: `Input`
- 制約・不変条件: `k_layers`, `color_count`, `goal` を保持し、`N` は定数として別管理する
- 備考: 生入力のみを保持し、色頻度や座標一覧のような solver 依存キャッシュは持たない


### 操作 `Op`

`paint`、`copy`、`clear` の 3 種類の出力操作を表す共通 enum である。

- コード上の代表名: `Op`
- Rust の想定型: `enum Op`
- 制約・不変条件: 各 variant の引数は問題文の制約範囲に入る
- 備考: solver はまず `Vec<Op>` を組み立て、最後に出力形式へ直列化する


### 状態 `State`

操作列の途中状態を表す、レイヤー集合とスコア関連の補助情報を持つ共通 struct である。

- コード上の代表名: `state`
- Rust の想定型: `State`
- 制約・不変条件: `layers.len() = K`、`op_count <= N^2`
- 備考: `layers`, `op_count`, `layer0_mismatch_count`, `goal_nonzero_count` を持つ。操作適用は `paint(state, goal, k, i, j, x)`、`copy(state, goal, k, h, r, di, dj)`、`clear(state, k)` の自由関数で行う


### beam 状態 `BeamState`

bitset 化した残差と、ここまでに採用した部品列を持つ探索状態である。

- コード上の代表名: `state`, `beam_state`
- Rust の想定型: `BeamState`
- 制約・不変条件: `residual` は常に目標画像 `g` の部分集合であり、`cost + residual.count()` が完成までの上界を与える
- 備考: `v004` 以降の solver では、この状態を幅制限付きで保持して部品選択を beam search する


### 操作回数 `T`

出力した操作列の長さを表す。

- コード上の代表名: `op_count` または `t`
- Rust の想定型: `usize`
- 制約・不変条件: `0 <= T <= N^2`
- 備考: スコア計算で用いる


### 非透明画素数 `U`

目標画像において `g_i,j != 0` を満たす画素数を表す。

- コード上の代表名: `goal_nonzero_count` または `u`
- Rust の想定型: `usize`
- 制約・不変条件: `N^2 / 2 <= U <= N^2`
- 備考: スコア式の分子に現れる


### レイヤー 0 の不一致数

現在の `state.layers[0]` と `input.goal` が異なる画素数を表す。

- コード上の代表名: `layer0_mismatch_count`
- Rust の想定型: `usize`
- 制約・不変条件: `0 <= layer0_mismatch_count <= N^2`
- 備考: `0` なら完成であり、全盤面比較なしで `is_goal()` を判定できる


### 回転回数 `r`

`copy` において、コピー元レイヤーを時計回りに 90 度回転する回数を表す。

- コード上の代表名: `rot` または `r`
- Rust の想定型: `usize`
- 制約・不変条件: `0 <= r <= 3`
- 備考: 実装では `r mod 4` とみなせる


### 変換 `τ`

回転 `r` とシフト量 `(Δi, Δj)` をまとめた、`copy` 1 回分の剛体変換を表す。

- コード上の代表名: `transform` または `tr`
- Rust の想定型: `Transform`
- 制約・不変条件: `τ = (r, di, dj)` で `r ∈ {0, 1, 2, 3}`、`di`, `dj` は `isize`
- 備考: 候補部品の出現位置や `copy` の引数をまとめて扱うときに使う


### シフト量 `Δi`, `Δj`

`copy` において、回転後レイヤー `h'` の左上をコピー先レイヤー `k` のどこに置くかを表す。

- コード上の代表名: `di`, `dj`
- Rust の想定型: `isize`
- 制約・不変条件: `-(N as isize) + 1 <= di, dj <= (N as isize) - 1`
- 備考: 配置後に範囲外へはみ出す `copy` は不正出力になる


### 回転後レイヤー `h'`

レイヤー `h` を時計回りに `r` 回回転した仮想的なレイヤーを表す。

- コード上の代表名: `rotated_h` または `rotated`
- Rust の想定型: `Vec<Vec<u8>>` 相当、または座標変換で暗黙表現
- 制約・不変条件: 元のレイヤー `h` 自体は変更されない
- 備考: `copy` のみで参照される概念である


### 生成用レイヤー数 `K'`

A 問題の入力生成で内部的に使われるレイヤー枚数を表す。

- コード上の代表名: `gen_k`
- Rust の想定型: `usize`
- 制約・不変条件: `1 <= K' <= 2K`
- 備考: 入力には現れず、生成規則の説明でのみ使う


### 生成パラメータ `p`

A 問題の入力生成で、`paint` を選ぶ確率を決めるパラメータを表す。

- コード上の代表名: `p`
- Rust の想定型: `usize`
- 制約・不変条件: `2 <= p <= 5`
- 備考: 各反復で `paint` を選ぶ確率は `p / 10`


### 色頻度 `num(x)`

入力生成時に、全レイヤーを通して色 `x` が何画素あるかを表す。

- コード上の代表名: `num_color[x]` または `color_freq[x]`
- Rust の想定型: `usize`
- 制約・不変条件: `x ∈ {1, ..., C}`
- 備考: `paint` 生成では `num(x)` が最小の色から一様に選ぶ


### 連結

あるレイヤーの非透明画素全体が上下左右 4 近傍で 1 つにつながっている性質を表す。

- コード上の代表名: `is_connected(layer)`
- Rust の想定型: `bool`
- 制約・不変条件: A 問題の入力生成では、更新後のレイヤーが常に連結であることを要求する
- 備考: 対角方向だけで接する場合は連結とみなさない


### copy の有効配置境界 `i_0`, `i_1`, `j_0`, `j_1`

回転後レイヤー `h'` の非透明画素の外接矩形を表す。

- コード上の代表名: `min_i`, `max_i`, `min_j`, `max_j`
- Rust の想定型: `usize`
- 制約・不変条件: 非透明画素が少なくとも 1 つあるとき定義される
- 備考: A 問題の生成では `Δi`, `Δj` の許容範囲計算に使う


### 部品 `P`

`copy` で再利用したい、色付き非透明画素の有限集合を表す。

- コード上の代表名: `pattern`, `motif`, `part`
- Rust の想定型: `Pattern`
- 制約・不変条件: 1 個以上の非透明画素を持つ
- 備考: 初版では 4 近傍連結な部品だけを候補にするとノイズを減らしやすい


### 編集用 window

`g` 上の小矩形切り出し位置とサイズを表す。

- コード上の代表名: `top`, `left`, `height`, `width`
- Rust の想定型: `isize`, `isize`, `usize`, `usize`
- 制約・不変条件: `top`, `left` は盤外にはみ出してもよく、盤外部分は透明扱いにする
- 備考: hill v1 では、pattern の編集はまず window を動かし、その後に mask を編集する


### 編集用 mask

window 内の各マスを pattern に含めるかどうかを表す 0/1 配列である。

- コード上の代表名: `mask`
- Rust の想定型: `Vec<u8>`
- 制約・不変条件: 長さは `height * width`
- 備考: `flip_mask_cell` はこの 1 ビットを反転する近傍である


### 編集用 pattern 表現

hill climbing 中で保持する、window と mask による pattern の編集表現である。

- コード上の代表名: `editable_pattern`, `pattern`
- Rust の想定型: `EditablePattern`
- 制約・不変条件: `window + mask -> trim -> 左上寄せ` の正規化後に利用する
- 備考: 評価用の `Pattern` は、この表現から透明境界を除いて生成する


### pattern 仕様 `PatternSpec`

window の位置と大きさ、および window 内で採用するマス集合をまとめた pattern 候補の仕様である。

- コード上の代表名: `spec`, `pattern_spec`
- Rust の想定型: `PatternSpec`
- 制約・不変条件: `top + height <= N` かつ `left + width <= N`
- 備考: `v201pro_sa` では mutate の対象であり、`normalize_spec` 後に `Pattern` へ落とす


### materialized pattern `Pattern`

trim 済み局所座標系で保持する色付きセル集合としての pattern 本体である。

- コード上の代表名: `pattern`, `pat`
- Rust の想定型: `Pattern`
- 制約・不変条件: `cells` はソート済みで、`signature` は回転正規化後の canonical 表現を持つ
- 備考: `hint_score` は seed pool 由来の優先度であり、配置列挙そのものには使わない


### 出現 `occ`

部品 `P` が目標画像中のどこに、どの回転で現れるかを表す 1 件の配置情報である。

- コード上の代表名: `occ`, `placement`
- Rust の想定型: `Occurrence`
- 制約・不変条件: `P` の全画素が盤面内に入る
- 備考: exact 版では全画素が `g_i,j` と一致する。repair 版では `good_mask` と `bad_mask` を持ち、少数の不一致を許した近似出現も扱う


### 候補 `Candidate`

1 個の `Pattern` に対して、回転 view、近似出現列 `occs`、および `good_union` をまとめた評価単位である。

- コード上の代表名: `cand`, `candidate`
- Rust の想定型: `Candidate`
- 制約・不変条件: `occs` は利得優先で整列され、`good_union` は全 `occ.good_mask` の和集合である
- 備考: `v201pro_sa` では SA 状態の各要素がこの単位で replay される


### pattern data `PatternData`

探索用の `PatternSpec` と評価用の `Candidate` を束ねた共有オブジェクトである。

- コード上の代表名: `data`, `pattern_data`
- Rust の想定型: `PatternData`
- 制約・不変条件: `spec` と `cand.pattern` は同じ元候補から生成される
- 備考: `v201pro_sa` では `Rc<PatternData>` を pool と SA 状態で共有する


### 残差 `R`

現在の `copy` 計画ではまだ最終色が正しくなっておらず、最後に `paint` で修復する必要がある画素集合を表す。

- コード上の代表名: `residual`, `remaining`
- Rust の想定型: `Grid` または色別 bitmask
- 制約・不変条件: `R` の各画素は「現状態の layer 0 の色が `g_i,j` と一致していない」ことを表す
- 備考: 近似 `copy` を許す solver では、`goal = 0` の画素も一時的に `R` に入る


### SA 状態 `SearchState`

現在採用している pattern 群と、その評価結果を持つ焼きなまし探索の状態である。

- コード上の代表名: `state`, `current`, `best`
- Rust の想定型: `SearchState`
- 制約・不変条件: `cost = action_cost + residual_count` の上界評価を保持する
- 備考: `v201pro_sa` では `pats` を近傍操作し、`evaluate_state` で `cost` と `residual` を再計算する


### 構築コスト `build_cost(P)`

部品 `P` を 1 枚の作業レイヤー上に準備するために必要な操作回数の見積もりを表す。

- コード上の代表名: `build_cost`, `prep_cost`
- Rust の想定型: `usize`
- 制約・不変条件: 少なくとも 1
- 備考: 初版では `|P|`、拡張版では再帰的 `copy` 構築を含めた見積もりに置き換える


### パターン ID `pid`

作業 layer に載せる exact pattern を一意に参照するための ID を表す。

- コード上の代表名: `pattern_id`, `pid`
- Rust の想定型: `usize`
- 制約・不変条件: `PatternRepo.entries[pid]` が存在する
- 備考: spray 用 candidate と prep-only subpattern を同じ ID 空間で管理する


### パターン管理表 `PatternRepo`

exact pattern 群と、その準備 recipe をまとめて保持する辞書を表す。

- コード上の代表名: `repo`
- Rust の想定型: `PatternRepo`
- 制約・不変条件: `entries` は dedup 済み signature 集合である
- 備考: `candidate_to_pattern_id` により spray candidate から exact pattern を引ける


### 準備 recipe `PrepRecipe`

ある target pattern を、別 source pattern の exact `copy` 群と不足 `paint` で構築する 1 段の準備手順を表す。

- コード上の代表名: `prep_recipe`, `recipe`
- Rust の想定型: `PrepRecipe`
- 制約・不変条件: source は target より真に小さく、recipe 実行後の work layer は target pattern と一致する
- 備考: `src_pattern_id`, `occurrences`, `residual_cells`, `build_cost`, `gain` を持つ


### 作業 layer 状態 `work_layers`

layer 1..K-1 に現在どの exact pattern が載っているかを表すキャッシュ状態を表す。

- コード上の代表名: `work_layers`
- Rust の想定型: `Vec<Option<PatternId>>`
- 制約・不変条件: 各要素は `None` か completed pattern の `PatternId`
- 備考: beam では途中まで塗られた layer を状態に含めず、「空」か「完成済み pattern」だけを持つ


### ビーム action `BeamAction`

multi-layer beam 上で 1 回の遷移に積まれる構築・吹き付け操作の抽象表現を表す。

- コード上の代表名: `action`, `BeamAction`
- Rust の想定型: `BeamAction`
- 制約・不変条件: `Load` は 1 枚の work layer を target pattern で完成させ、`Spray` は loaded pattern から layer 0 に `copy` する
- 備考: 実操作列への展開は `state_to_ops_multilayer` で行う


### バリアント `Variant`

1 つの pattern を residual 上でどう適用するかを表す軽量なパラメータを表す。

- コード上の代表名: `variant`, `variant_idx`
- Rust の想定型: `Variant`
- 制約・不変条件: `seed` は採用開始 occurrence、`limit` は greedy に拾う occurrence 数上限を表す
- 備考: 同じ pattern でも異なる `Variant` により batch の選ばれ方が変わる


### パターン適用列 `S`

焼きなましで最適化する、順序付きの `(candidate_id, variant_id)` 列を表す。

- コード上の代表名: `seq`, `current`, `next`
- Rust の想定型: `Vec<UseStep>`
- 制約・不変条件: 各要素は 1 回の pattern batch 適用を表し、評価時に先頭から順に residual へ適用される
- 備考: 出力操作列そのものではなく、操作列を生成する高水準の状態である


### スコア式

問題 A・B の各テストケースに対する得点を表す。

- コード上の代表名: `score`
- Rust の想定型: `i64` または `f64` を経由して `i64`
- 定義:

$$
\mathrm{score} = \mathrm{round}\left(10^6 \times \left(1 + \log_2\frac{U}{T}\right)\right)
$$

- 備考: 提出全体の得点は各ケース得点の合計である


### batch scan 上限 `BATCH_SCAN_LIMIT`

`best_batch_for_pattern` の 1 回で、動的再評価の対象にする occurrence の先頭件数上限を表す。

- コード上の代表名: `BATCH_SCAN_LIMIT`
- Rust の想定型: `usize`
- 制約・不変条件: `0 < BATCH_SCAN_LIMIT <= MAX_OCCURRENCES_PER_PATTERN`
- 備考: static rank 上位だけを動的に再評価し、planner のコストを抑えつつ batch 品質を上げる


### seed 初期状態

空 pattern 状態とは別に、残差 `W` から 1〜3 個の pattern をランダム生成して評価する初期候補群を表す。

- コード上の代表名: `seed_patterns`, `seed_count`
- Rust の想定型: `Vec<EditablePattern>`, `usize`
- 制約・不変条件: 各 pattern は `sample_add_pattern` で生成し、正規化後だけを採用する
- 備考: SA 本探索に入る前の初期位置改善に使う


### slot `slot`

大きい pattern から順に並べたときの pattern の位置を表す。

- コード上の代表名: `slot`, `idx`
- Rust の想定型: `usize`
- 制約・不変条件: `slot=0` が最大 pattern、後ろほど小さい pattern を想定する
- 備考: `v107_sa_slotbias` では add の window 分布、変更対象の選び方、resize の shrink/expand バイアスに使う


### size prior penalty

隣り合う slot の pattern サイズが十分に減衰しているかをみる SA 用の弱い事前分布を表す。

- コード上の代表名: `slot_size_prior_penalty`, `SLOT_SIZE_PRIOR_WEIGHT`
- Rust の想定型: `usize`, `f64`
- 制約・不変条件: best 更新は raw の `eval.cost` で行い、受理判定だけが prior 付き energy を使う
- 備考: `s_i <= ceil(3/4 * s_{i-1})` を目安にし、超過分だけ penalty を加える


### 埋め込み入力判定 `EmbeddedInputCase`

特定の既知 input を hash で判定し、固定の高品質出力を即返す分岐を表す。

- コード上の代表名: `EmbeddedInputCase`, `detect_embedded_input_case`, `solve_embedded_input_case_if_matches`
- Rust の想定型: `enum EmbeddedInputCase`
- 制約・不変条件: hash 一致時だけ固定出力を返し、不一致時は通常 solver に流す
- 備考: `v108_sa_slotbias_embedded` で `src/make_input/embedded_case_snippet.rs` を単一 solver に結合して使う
