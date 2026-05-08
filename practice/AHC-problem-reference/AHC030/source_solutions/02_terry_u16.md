# THIRD プログラミングコンテスト2023 (AHC030) 解説 / terry_u16

## 参照元

- 解説記事: <https://www.terry-u16.net/entry/ahc030>
- 著者: terry_u16
- 成績: 記事冒頭で `2,140,937,483,257` 点、優勝と記載されている。
- コード: あり。記事中の「コンテスト中の日記」リンク先が GitHub リポジトリ <https://github.com/terry-u16/ahc030> で、`src` 以下に Rust 実装が公開されている。最終提出と完全一致かはリポジトリ上だけでは断定できないが、記事で説明されている単点掘り・複数マス占い・パラメータ補間・AVX 高速化を含む実装を読めた。

## 解法の全体像

隠れ状態を「各油田の左上位置の配列」と見なし、過去の観測に対して尤もらしい配置を多数生成する。その候補集合を、真の事後分布の近似として扱う。

大きく 2 系統のソルバを持つ。

- `SingleDigSolver`: 1 マス掘り中心。掘った正確な値と矛盾しない配置を乱択 BFS 風に集め、次に掘る 1 マスを相互情報量で選ぶ。
- `MultiDigSolver`: 複数マス占い中心。占い履歴に対する対数尤度の高い配置を状態プールから生成し、相互情報量 / コストが大きいマス集合を山登りで作って占う。

どちらを使うか、各ターンの時間配分、状態生成とクエリ選択の時間比、相互情報量計算に使う候補数、回答閾値などは、`N, M, epsilon, 平均油田サイズ` からガウス過程回帰で推定したパラメータで切り替える。

## 主要アイデア

### 候補配置を全列挙の代替にする

全配置は概ね `N^(2M)` 通りで列挙できないため、高尤度または低ペナルティの配置だけを状態プールに貯める。状態は Zobrist hash で重複排除し、同じ形の油田が複数ある場合は shift を正規化して同一状態をまとめる。

単点掘りでは、状態の評価は「掘った値との矛盾量」である。複数マス占いでは、評価は占い結果の対数尤度である。

### 破壊再構築で状態を増やす

状態生成は記事の「謎乱択 BFS」に対応する。プールから状態を重み付きに取り出し、油田を数個選んで配置し直す。単点掘りでは違反量が小さくなる位置を貪欲に選ぶ。複数マス占いでは、置き直し候補の対数尤度を計算し、尤度比に比例した重み付き乱択で位置を選ぶ。

疑似コードは次の形である。

```text
states = initial_pool
while time remains:
    base = sample(states, weight = likelihood_or_penalty_weight)
    moved = choose several oil ids
    for oil in moved:
        remove oil, put it temporarily at random legal shift
    for oil in moved:
        remove oil
        score every legal shift by resulting likelihood
        add oil at weighted-random high-likelihood shift
    normalize equal-shape oils
    if hash is new:
        add to states
```

### クエリ集合を相互情報量で選ぶ

候補配置集合と確率重みがあれば、ある占い集合 `S` を投げたときの返値分布と、返値後の条件付きエントロピーを近似計算できる。複数マス占いのコストは `1/sqrt(|S|)` なので、実装上のスコアはおおむね

```text
score(S) = mutual_information(X; observed_value) * sqrt(|S|)
```

である。`S` は全探索できないので、最尤状態と次点状態の油マスの和集合を初期解にし、1 マス追加/削除、1 マス入れ替えに相当する flip 近傍で山登りする。コード上は速度優先で焼きなましの温度採択ではなく、改善したときだけ受理する形になっている。

### 事後確率ではなく回答集合もまとめる

最終回答は油田配置ではなく `v>0` のマス集合である。`MultiDigSolver` は回答判定時、尤度上位の状態を `v>0` の bitset で group 化し、同じ回答集合を作る配置の尤度を合算している。1 位回答集合と 2 位回答集合の尤度比が閾値を超えたら回答する。

## 最終コードの構造

### 状態表現

- 共通の状態は `shift: Vec<CoordDiff>` で、各油田の平行移動量を持つ。
- `hash: u64` は油田番号と shift に対する Zobrist hash の xor。
- `to_answer` は各油田の配置マスを OR し、`v>0` の座標列を作る。
- 同形状油田は `normalize` で shift をソートし、順序違いだけの重複を潰す。

単点掘りの `State` は、観測済みマスに残る差分 `map: Map2d<Option<i32>>` と `violations` を持つ。油田を置くと観測値から 1 を引き、絶対値和を矛盾量にする。

複数マス占いの `State` は、各観測に対する真の重なり数 `counts`、AVX 用の `counts_u32`、`log_likelihood` を持つ。油田を 1 個追加/削除すると、その油田配置が各観測集合と何マス重なるかだけを足し引きして尤度を差分更新する。

### 観測/制約

`Observation` は、占った座標集合 `pos` と、真の総和 `t` ごとの `log_likelihoods[t]` を持つ。`k=|S|`、観測値を `x` として、

```text
mean = (k - t) * eps + t * (1 - eps)
variance = k * eps * (1 - eps)
P(observed=x | true=t)
  = Normal(mean, variance) が丸め後 x になる確率
```

を正規分布 CDF で計算する。`k=1` の単点掘りは正確なので、観測値と一致する `t` のみ確率 1 とする。

`ObservationManager` は、各油田 `i` と各 shift が各観測集合に何マス重なるかを `relative_observation_cnt[i][shift][obs]` として蓄える。これにより、近傍評価で毎回盤面全体を走査せずに済む。

### 評価関数

単点掘り:

```text
violation = sum over observed cells |observed_value - value_from_state|
```

複数マス占い:

```text
log_likelihood(state) =
    sum_obs observation[obs].log_likelihoods[ true_sum(state, obs.S) ]
```

状態プールからのサンプリングや置き直し候補の選択では、最大対数尤度を引いて `exp(log_likelihood - max_log_likelihood)` を使い、アンダーフローを避けている。

### 探索

単点掘りの状態生成は、複数油田を選び、ランダム位置に一度置き直した後、各油田を違反量が最小になる合法位置へ貪欲に置き直す。最小違反量が更新されたら、その違反量の状態だけを残す。

複数マス占いの状態生成は、状態を尤度重みで取り出し、2-5 個程度の油田を選ぶ。各油田の候補 shift について `add_oil_whatif` で全観測の尤度を評価し、尤度比を重みにして shift を乱択する。内側は AVX2 gather を使い、`obs_log_likelihoods[offset + current_count + added_count]` をまとめて足している。

### クエリ選択

単点掘りでは、候補状態を最大 200 個サンプルし、各未観測マスについて `v` の値ごとに候補数を数える。掘った後の条件付きエントロピーが最小になるマスを選ぶ。

複数マス占いでは、`sampler::select_sample_points` が次の流れで集合を選ぶ。

```text
states = likelihood top states, then truncate by max_entropy_len and probability mass
initial S = oil cells of best state union oil cells of second state
while time remains:
    S' = flip one selected/unselected cell, or swap selected and unselected cells
    if score(S') >= score(S):
        S = S'
return S
```

条件付きエントロピー計算では、各候補状態 `x` について `true_sum_x(S)` を持ち、`P(y|x,S)` を `ProbTable` から取り出す。`P(y)=sum_x P(x)P(y|x,S)` と `P(x,y)` から `H(X|Y)` を計算し、`H(X)-H(X|Y)` を相互情報量にする。返値範囲は正規分布の質量がほぼ入る範囲だけを lazy に作る。

### 回答判定

`MultiDigSolver` は、上位状態を `v>0` bitset でまとめ、同じ回答集合になる状態の相対尤度を合算する。最有力集合と次点集合の比が `answer_threshold_ratio` 以上なら回答する。残りクエリ数が少ない場合は、状態プールの候補を順に回答するフォールバックもある。

`SingleDigSolver` は、違反量 0 の状態が 1 個だけになったら回答する。最後まで確定しなければ、掘って分かっている `v>0` マスを回答する実装になっているが、これはほぼ非常時用と見るべきである。

### 時間配分

`TimeConductor` が各ターンの締切時刻を作る。正規化ターン `x=t/max_turn` に対して、重みを

```text
(1 - x)^k + b*x
```

とし、残り時間を累積重みに比例配分する。複数マス占いでは、ターン内時間の `phase_ratio` を状態生成に使い、残りをクエリ集合の山登りに使う。`k, b, phase_ratio` などはガウス過程回帰で推定される。

## 実装上重要な断片

観測確率テーブル:

```text
for true_sum in 0..=total_oil_tiles:
    if k == 1:
        logp[true_sum] = 0 if true_sum == observed else -inf
    else:
        mu = (k - true_sum) * eps + true_sum * (1 - eps)
        sigma = sqrt(k * eps * (1 - eps))
        p = CDF(observed + 0.5) - CDF(observed - 0.5)
        logp[true_sum] = ln(max(p, tiny))
```

複数マス占いの回答集合ごとの尤度集約:

```text
for state in high_likelihood_states:
    answer_bitset = union of shifted oil bitsets
    grouped[answer_bitset] += exp(state.log_likelihood - best.log_likelihood)
if grouped[best] / grouped[second] >= threshold:
    answer(best)
```

相互情報量評価:

```text
base_H = -sum_x p[x] log2 p[x]
for x in states:
    s = true_sum_x(S)
    for y in likely_observed_values(k, s):
        joint[x,y] = p[x] * P(y | x, S)
        obs[y] += joint[x,y]
cond_H = -sum_{x,y} joint[x,y] * log2(joint[x,y] / obs[y])
score = (base_H - cond_H) * sqrt(|S|)
```

## この解法の本質

「あり得る配置をサンプリングで事後分布として近似し、その分布のエントロピーを最も安く減らす観測を選ぶ」ことが本質である。問題はベイズ推定と情報量最大化に見えるが、実際には高尤度配置をどれだけ速く多様に集められるか、相互情報量をどれだけ速く評価できるか、ケースごとに時間と閾値をどう割り振るかが勝負になっている。

特に強い点は、最終回答が配置ではなくマス集合であることを利用し、配置の尤度ではなく回答 bitset ごとの尤度比で決断している点である。

## 真似するならまず実装する部分

1. 油田ごとの合法 shift と、shift 済み bitset を作る。
2. 観測 `S, observed` に対して `true_sum -> log likelihood` を計算する。
3. 状態を `shift 配列 + 各観測の true_sum + log_likelihood` で持ち、油田 1 個の移動を差分更新する。
4. 破壊再構築で高尤度状態をプールに貯める。
5. 上位候補だけで `v>0` 回答集合の尤度比を計算して回答する。
6. その後、相互情報量 `I * sqrt(k)` によるクエリ集合選択を追加する。

最初から AVX2 やガウス過程回帰まで入れる必要はない。まずは対数尤度、状態プール、回答集合の集約が動けば、この解法の芯に到達できる。

## 注意点・未理解点

- GitHub コードは読めたが、AtCoder の最終提出と完全一致するかは確認できない。リポジトリには日記と実装があり、最終提出 ID への直接リンクは記事中に見当たらなかった。
- `Params` の巨大な base64 配列は、記事で述べられている optuna + ガウス過程回帰の結果を埋め込んだものだと読めるが、元のチューニングデータや学習手順まではリポジトリだけでは完全には追っていない。
- `sampler` の初期解は最尤状態と次点状態の油マス和集合で、記事の日記にも「偶然実装ミスったやつが当たった」とある。なぜこれが普遍的に強いかは理論だけでは説明しきれない。
- `SingleDigSolver` の最終フォールバックは観測済み正マスだけを答えるため、通常の正解経路としては危険である。実際には違反量 0 の単一候補に到達する前提の保険と見る。
- AVX2 前提の高速化が入っているため、移植時は CPU 機能や fallback を意識する必要がある。
