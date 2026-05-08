# AHC030 Polyomino Mining 統合解法メモ

## このメモの位置づけ

このメモは、参照元ごとの精読メモを統合し、実装者が solver を作るための方針に落としたものだ。単なる解説の要約ではなく、「どの状態を持つか」「何を評価するか」「まず何を実装するか」を明示する。

個別メモは次に分けてある。

- [01_official_wata.md](source_solutions/01_official_wata.md): 公式解説。ベイズ推定、対数尤度、相互情報量、焼き鈍しの理論。
- [02_terry_u16.md](source_solutions/02_terry_u16.md): 優勝解法。単点掘り/複数マス占い、候補プール、相互情報量、回答集合ごとの尤度集約。
- [03_eijirou.md](source_solutions/03_eijirou.md): 3位解法。焼きなまし状態生成、差分尤度更新、32候補、回答閾値、実装構造が具体的。
- [04_kiri8128.md](source_solutions/04_kiri8128.md): 候補状態集合を使った情報量近似と回答確率。
- [05_utac.md](source_solutions/05_utac.md): 1マス掘り中心の候補削除・制約伝播。最小実装として再現性が高い。

## 問題の本質

隠れ状態は「各油田の平行移動量」である。状態 `x` が決まれば、全マスの埋蔵量 `v_x(i,j)` と任意集合 `S` の総和 `v_x(S)` が決まる。観測は次の2種類である。

- 1マス掘り: `v(i,j)` が正確に分かるが、コストは `1`。
- 複数マス占い: `v(S)` のノイズ付き観測が安く得られる。コストは `1/sqrt(|S|)`。

最終的に答えるのは配置 `x` ではなく、`v(i,j)>0` のマス集合である。したがって、配置が一意でなくても、候補配置が同じ正マス集合を作るなら回答できる。

この問題でスコアを良くするレバーは次である。

1. 少ない観測で高尤度な配置候補を集める。
2. 候補集合をよく分割する複数マス占いを選ぶ。
3. 最終回答は最尤配置だけでなく、同じ回答集合を作る候補の確率和で判断する。
4. 1マス掘りは主軸ではなく、候補削除や最終検証に使う。

高コストな全確認は正解保証にはなるが、平均スコアは悪い。この問題では、正解保証より「観測で事後分布を狭める」方針を主解法にする。

## 参照解法の比較

| 参照元 | 主方針 | 実装難度 | 真似すべき要素 |
|---|---|---:|---|
| 公式/wata | ベイズ推定 + 相互情報量 + 焼き鈍し | 高 | 尤度式、`I(X;Y)*sqrt(k)`、差分更新 |
| terry_u16 | 高尤度状態プール + MIクエリ + 回答集合尤度集約 | 最高 | 回答 bitset ごとの尤度比、破壊再構築、観測確率テーブル |
| eijirou | 8状態SA + 32候補 + 近似クエリ選択 | 高 | 実装しやすい状態構造、差分尤度更新、初期占い |
| Kiri8128 | 候補状態集合 + 情報量近似 | 中 | 候補を割るクエリ選択、マスごとの油田確率 |
| uta_ccc | 1マス掘り + 配置候補削除 | 低 | `v=0` による候補削除、最小実装としての堅さ |

推奨する主ルートは、**eijirou型の「対数尤度付き候補プール」** である。理由は、上位解法の本質であるベイズ推定に近く、かつ terry_u16 型より実装量が少ないからである。相互情報量の完全実装は後回しにし、まずは固定/近似クエリでもよいので、高尤度候補を作って回答集合の確率で判断できるところまで作る。

最小実装としては uta_ccc 型の候補削除を使えるが、これは主解法ではなく、時間が足りない場合や終盤検証の補助とみなす。

## 推奨実装ルート

### Step 1: 配置と観測の土台

まず、以下を実装する。

```text
shape[m]            : 油田 m の相対座標
placements[m][p]    : 油田 m の合法配置 p が覆うマス集合
queries[q]          : 占い集合 S_q と返値 r_q
dig_values[cell]    : 1マス掘りで得た値。未観測なら none
```

盤面は最大 `20*20=400` マスなので、マス集合は bitset で持つとよい。Rustなら簡易実装では `Vec<u64>`、最初は `Vec<usize>` でも動く。

配置列挙:

```text
for each oil m:
    h = max_i(shape[m]) + 1
    w = max_j(shape[m]) + 1
    for oi in 0..=N-h:
        for oj in 0..=N-w:
            cells = {(oi+di, oj+dj) | (di,dj) in shape[m]}
            placements[m].push(cells)
```

### Step 2: 観測確率 `log P(r | true_sum)` を作る

複数マス占いで、集合サイズ `k`、真の総和 `s`、返値 `r` の確率を `q(k,s,r)` とする。

```text
mu = (k - s) * eps + s * (1 - eps)
sigma = sqrt(k * eps * (1 - eps))

if r == 0:
    q = Pr[X < 0.5]
else:
    q = Pr[r - 0.5 <= X < r + 0.5]
```

実装では `logq = ln(max(q, 1e-300))` を使う。1マス掘りは正確なので、真値が観測値と一致する場合だけ `0`、違う場合は大きな負値にする。

`s` の最大は全油田面積の合計以下でよい。雑に `N*N*M` まで取ると重いので、`total_area = sum shape[m].len()` を使う。

### Step 3: 状態を「配置列 + 各クエリ総和 + 対数尤度」で持つ

eijirou/terry 型の核である。

```text
State:
    pos[m]       : 油田 m の配置ID
    sums[q]      : この状態における v(S_q)
    log_like     : sum_q log P(r_q | sums[q])
```

状態の初期化:

```text
random_state():
    for m:
        pos[m] = random legal placement id
    rebuild sums[q] from placements
    log_like = sum logq[q][sums[q]]
```

最初は全再計算でよい。速度が足りなくなったら、次の差分更新に進む。

### Step 4: 油田1個移動の差分更新

高尤度候補を集めるには、状態を大量に動かす必要がある。毎回盤面を作り直すと遅いので、各配置が各クエリに何マス寄与するかを前計算する。

```text
contrib[m][p][q] = | placements[m][p] ∩ queries[q].S |
```

油田 `m` を `old -> new` に動かすとき:

```text
delta_log_like = 0
for q in queries:
    old_s = state.sums[q]
    new_s = old_s - contrib[m][old][q] + contrib[m][new][q]
    delta_log_like += logq[q][new_s] - logq[q][old_s]
if accept(delta_log_like):
    state.sums[q] = new_s for all q
    state.pos[m] = new
    state.log_like += delta_log_like
```

まずは `O(M * placements * queries)` の前計算でもよい。クエリ数は最大でも `2N^2` だが、良いsolverではそこまで使わない。

### Step 5: 焼きなましで高尤度候補を集める

目的は最尤1点だけでなく、事後分布の代表候補を複数集めることである。候補数は最初 `32` を目安にする。

```text
build_candidates(time_budget):
    pool = empty top-K set
    states = 8 random states or previous candidates
    while time remains:
        for state in states:
            m = random oil
            new_pos = random legal position
            gain = delta_log_like(state, m, new_pos)
            temp = schedule()
            if gain >= 0 or rng() < exp(gain / temp):
                commit move
            insert state into pool if unique
    return top 32 states by log_like
```

近傍は最初は次だけでよい。

- 3/4: 1油田を上下左右に1マス動かす。合法でなければ無視。
- 1/4: 1油田をランダム合法位置へ移す。

上位化するなら、eijirou の「2油田入れ替え」や terry_u16 の「複数油田を外して重み付き再配置」を追加する。

### Step 6: 候補上で回答集合の確率を集約する

回答対象は配置ではなく正マス集合である。最尤状態だけで回答するのではなく、同じ正マス集合を作る候補の重みを足す。

```text
group_answers(candidates):
    best_log = max state.log_like
    grouped = map answer_bitset -> weight
    for state in candidates:
        ans = OR of placements[state.pos[m]]
        w = exp(state.log_like - best_log)
        grouped[ans] += w
    sort grouped by weight descending
```

回答条件:

```text
if grouped[0].weight / grouped[1].weight >= threshold:
    answer grouped[0].bitset
```

初期閾値は `10.0` から `100.0` 程度でよい。候補が少ない、観測が少ない、`epsilon` が大きい場合は慎重にする。掘って `v>0` と分かっているマスは必ず回答集合に含める。

### Step 7: 次の複数マス占いを選ぶ

本来は相互情報量 `I(X;Y) * sqrt(|S|)` を最大化する。最初から完全実装が重ければ、eijirou/Kiri 型の近似で始める。

最小版:

```text
choose_query(candidates):
    best = candidates[0]
    expected_v[cell] = weighted average v_x(cell) over candidates
    S = { cell | v_best(cell) > expected_v[cell] }
    if |S| < 2:
        S = uncertain cells where weighted positive probability is near 0.5
    return S
```

少し良い版:

```text
score_query(S):
    for state in candidates:
        s[state] = v_state(S)
    return entropy/distribution spread of s[state] times sqrt(|S|)

choose_query:
    start from best-vs-expected set
    repeat:
        try remove one cell
        try add one cell
        try swap one selected and one unselected cell
        keep if score improves
```

本命版:

```text
score_query_mi(S):
    H_before = entropy(weights)
    for each possible observed y:
        py = sum_x weight[x] * P(y | x, S)
        posterior[x] = weight[x] * P(y | x, S) / py
        H_after += py * entropy(posterior)
    return (H_before - H_after) * sqrt(|S|)
```

最小版でも「候補を分離する集合」を選ぶことが重要である。行・列・ブロックなどの固定占いだけでは、候補に応じた観測にならない。

### Step 8: 1マス掘りの使い方

1マス掘りは高コストなので、全確認の主軸にしてはいけない。使い方は次に限定する。

- `v=0` が出ると多くの配置を削れるマスを掘る。
- 回答候補に含めるかどうか、候補上で割れているマスを掘る。
- 回答失敗後、境界マスを確認する。
- `epsilon` が大きく、複数マス占いだけでは候補が割れないときに補助する。

utac 型の候補削除は実装しやすい。

```text
on_dig(cell, value):
    dig_values[cell] = value
    if value == 0:
        for m,p:
            if placements[m][p] contains cell:
                alive[m][p] = false
    else:
        # 安全な場合だけ制約伝播
        coverable_oils = {m | exists alive p covering cell}
        if len(coverable_oils) == value:
            for m in coverable_oils:
                keep only placements covering cell
```

この `alive` は、焼きなましのランダム位置候補を絞るのにも使える。

## 推奨する最小solver

sub agentに実装させるなら、まず次の版を目標にする。

```text
1. 合法配置とbitsetを作る
2. 初期占いを数回行う
   - 全盤面
   - 上半分/下半分
   - 左半分/右半分
   - 必要なら4分割
3. 観測ごとの logq テーブルを作る
4. ランダム状態を複数作り、焼きなましで上位32候補を集める
5. 候補上で回答bitsetをgroup化する
6. 回答集合の1位/2位重み比が大きければ回答する
7. そうでなければ、最尤状態と候補期待値の差から集合Sを作って占う
8. 回答失敗時は、その集合を失敗済みとして避けるか、不確実マスを数個掘る
9. 残り操作が危険なら、候補上で確率が高い回答集合を順に試す
```

これで平均スコアが全確認相当なら、失敗原因は次のどれかである。

- 候補生成が真の高尤度領域を拾えていない。
- クエリが固定的で、候補を分離していない。
- 回答判定が最尤1点に寄りすぎている。
- 観測確率の丸め処理が間違っている。
- 1マス掘りの使い方が多すぎる。

## 実装パラメータの初期値

まずは次の値でよい。

```text
candidate_pool_size = 32
parallel_states = 8
initial_temperature = 5.0
final_temperature = 0.5
answer_ratio_threshold = 30.0
max_query_hillclimb_trials = 200
max_candidates_for_query = 32
initial_surveys = 全盤面 + 2分割 + 4分割の一部
```

`log_like` のスケールが自然対数なら温度も自然対数スケールに合わせる。採択率が極端に低い場合は温度を上げる。

## やってはいけない方針

- 全マスを1マス掘りしてから回答する方針を主解法にしない。
- 固定パターンの占いだけを大量に投げ、候補集合に応じたクエリ選択をしない。
- 最尤状態1個だけで確信判定する。回答集合ごとの重み集約を使う。
- ノイズ付き占いを「返値と真値が一致するか」だけで判定する。正規分布の尤度を使う。
- 候補状態をランダム生成しただけで終える。観測履歴に対する尤度で改善する必要がある。

## 実装チェックリスト

- `r=0` の観測確率を `Pr[X < 0.5]` として扱っているか。
- `logsumexp` または最大尤度を引いた重み計算を使っているか。
- 回答集合には、掘って `v>0` と分かったマスを必ず含めているか。
- 占い集合に重複座標がないか。
- `s = v(S)` の上限を十分に取っているか。
- 同形状油田が複数ある場合、状態重複が増えやすい。余裕があれば配置列を正規化する。
- 回答失敗した集合を再度そのまま出していないか。

## 改善順序

1. まず `logq` と候補状態の尤度計算が正しいかを確認する。
2. 次に、初期占い後の最尤候補が真値に近いかを可視化・ログで確認する。
3. 候補生成が弱ければ、SA時間、候補数、近傍、温度を調整する。
4. クエリが弱ければ、固定占いを減らし、候補を分ける集合探索を入れる。
5. 回答失敗が多ければ、回答bitsetごとの尤度比閾値を上げる。
6. 終盤だけ1マス掘りで不確実マスを検証する。
7. 速度が足りなければ `contrib[m][p][q]` と bitset popcount の差分更新を入れる。

## 期待スコア帯の目安

ローカル評価では、単純な全確認型は平均 `2e8` 程度になりやすい。これは失敗水準とみなす。

目標の目安:

- 候補削除型の最小実装: 全確認より明確に良いこと。
- 固定占い + 尤度付き候補プール: `1e8` 未満をまず狙う。
- 候補応答に応じたクエリ選択 + 回答集合尤度集約: `5e7` 未満を狙う。
- 差分SA + 相互情報量 + パラメータ調整: `1e7` 台を狙う。

この目安は実装環境とテストセットに依存するため、絶対値ではなく「全確認からどれだけ離れたか」を見る。

## 参照元

- [AHC030 解法概略](https://img.atcoder.jp/ahc030/ahc030.pdf)
- [THIRD プログラミングコンテスト2023 (AHC030) 解説](https://www.terry-u16.net/entry/ahc030)
- [terry-u16/ahc030](https://github.com/terry-u16/ahc030)
- [AHC030 参加記](https://eijirou-kyopro.hatenablog.com/entry/2024/02/22/152604)
- [AtCoder Submission #50450879](https://atcoder.jp/contests/ahc030/submissions/50450879)
- [AHC 030](https://kiri8128.hatenablog.com/entry/2024/02/19/211740)
- [AtCoder Submission #50441823](https://atcoder.jp/contests/ahc030/submissions/50441823)
- [AHC030参加記 最終149位](https://utac.hateblo.jp/entry/2024/02/19/232118)
- [AtCoder Submission #50414729](https://atcoder.jp/contests/ahc030/submissions/50414729)
