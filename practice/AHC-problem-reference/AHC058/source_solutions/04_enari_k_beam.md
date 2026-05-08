# AHC058 - enari_k_beam 解法メモ

## 参照元

- 記事: https://zenn.dev/enari_k/articles/684e5246db0b5d
- 著者: enari_K
- サイト: Zenn
- 種別: 実装解説、提出コード付き参加記
- 成績・順位: 全体 389 位、水色パフォーマンス
- コード有無: あり。記事本文に C# コードが埋め込まれている
- コードを読めたか: 読めた。記事本文の埋め込みコードを HTML から抽出して確認した
- 読めなかったもの: AtCoder の提出リンクや外部 GitHub リポジトリは記事内にない。記事中コードが最終提出そのものか、説明用に整理されたものかは本文以上には確認できない

## 解法の全体像

ターンごとに状態を複数保持するビームサーチである。各状態から、待機と有望な強化候補上位 6 個だけを展開する。候補生成では、残りターンに対する近似将来価値からコストペナルティを引き、正の候補だけを残す。状態評価では、現在のりんご数に「以降何もしなかった場合の将来価値」を加える。ビーム幅は基本 600 だが、終盤や実行時間 1850ms 超過後は幅を縮める。

## 主要アイデア

- 状態を `K`, `B`, `P` と親参照で持ち、ビームサーチで時系列意思決定を扱う。
- 全 40 種類の強化を毎回すべて展開せず、近似利益が正のものから上位 6 個に絞る。
- 将来価値を `t^level / factorial` 風の係数で近似し、累乗を `powerCache` に前計算する。
- コストに `1.5` 倍のペナルティを掛け、手元資金を失う不利を評価に入れる。
- 残りターンが少ないときや時間が厳しいときはビーム幅を小さくして TLE を避ける。
- 出力列は各状態の親ポインタと直前アクションから復元する。

## 最終コードの構造

### 状態表現

- `State.K`: 現在のりんご数。`BigInteger`。
- `State.B`: 機械数。`BigInteger[L,N]`。
- `State.P`: 強化回数。`int[L,N]`。
- `State.Parent`: 親状態への参照。
- `State.ActionI`, `State.ActionJ`: 親からこの状態へ来た行動。`-1` は待機。
- `State.EvaluationScore`: ビーム内ソート用の `double` 評価値。
- `Clone()` では `K`, `B`, `P`, 評価値をコピーし、履歴は親参照で節約する。

### 観測・制約・入力の扱い

- `Console.In.ReadToEnd().Split(...)` で全入力を読み、`N`, `L`, `T`, 初期 `K`, `A`, `C` を読む。
- `C` と `B` と `K` は `BigInteger`。
- 初期状態は全 `B[i,j]=1`, 全 `P[i,j]=0`。
- 支払い可能な候補のみ強化候補に入れる。
- 候補適用時は、先にコストを支払い `P` を 1 増やし、その後 1 ターン分の生産を行う。

### 評価関数

- 定数:
  - `COST_PENALTY = 1.5`
  - `LEVEL_WEIGHTS = {1.0, 2.0, 6.0, 24.0}`
  - `BASE_BEAM_WIDTH = 600`
  - `TOP_ACTIONS = 6`
- 前計算:
  - `powerCache[t,p] = t^p / LEVEL_WEIGHTS[p-1]`
- 候補生成のスコア:
  - ID ごとに `P` の累積積を作る。ただし候補生成では `P=0` を `1` とみなして、未起動の上位 Level も候補に乗りやすくしている。
  - `gain = A[j] * cumulative_P_to_level_i * powerCache[remainingTurns, i+1] * B[i,j]`
  - `score = gain - cost * COST_PENALTY`
  - `score > 0` の候補だけを残す。
- 状態評価:
  - `futureValue = sum_j sum_n A[j] * chainP(n,j) * B[n,j] * powerCache[remainingTurns,n+1]`
  - `chainP` は実際の `P` の積なので、途中に 0 がある系列は寄与しない。
  - `EvaluationScore = K + futureValue`

### 探索・構築・更新

- `currentBeam` を初期状態 1 つで開始する。
- 各ターンで残りターンからビーム幅を決める。
- 各状態について `GetPromisingActions` を呼ぶ。
- 候補ごとに状態を clone し、行動適用、生産、評価を行う。
- 生成した全候補を評価値降順で sort し、上位 `currentBeamWidth` 個を次ビームにする。
- ハッシュによる重複排除は記事コードには見当たらない。公式解説のビームより素朴な形である。

### 操作・クエリ・出力選択

- `GetPromisingActions` は待機 `(-1,-1)` を常に候補に入れる。
- 残りターンが 1 以下なら待機のみ返す。
- 強化候補は利益正の上位 6 個。
- 最終ターン後、`currentBeam[0]` を最良状態とし、親参照を辿って行動を stack に積み、先頭から 500 行出力する。

### 時間配分・パラメータ

- 基本ビーム幅は 600。
- 残り 10 ターン未満ならビーム幅 1。
- 残り 50 ターン未満なら、`max(10, 600 * remainingTurns / 50)` に縮小する。
- 経過時間が 1850ms を超えたらビーム幅 1。
- `TOP_ACTIONS=6` により 1 状態あたりの展開数を待機込みで最大 7 程度に抑える。
- `LEVEL_WEIGHTS` は階乗ベースだが、記事コメントでは調整によりスコアが大きく変わるとされている。

## 実装上重要な断片

```text
get_promising_actions(state, remaining):
    actions = [wait]
    for j in ids:
        cumulative = product of max(P[k][j], 1) by level
        for i in levels:
            if cost(i,j) is affordable:
                gain = A[j] * cumulative_to_i * powerCache[remaining][i+1] * B[i][j]
                score = gain - cost(i,j) * 1.5
                if score > 0:
                    add (i,j,score)
    return wait + top 6 upgrades by score
```

```text
beam_search:
    beam = [root]
    for turn in 0..T-1:
        width = width_by_remaining_turns_and_elapsed_time()
        next = []
        for state in beam:
            for action in get_promising_actions(state, T-turn):
                child = clone(state)
                apply action if upgrade
                simulate_production(child)
                child.score = child.K + approximate_future_value(child)
                next.add(child)
        beam = top width states
    output path of beam[0]
```

## この解法の本質

この解法は、完全な探索ではなく「有望な購入だけを残す候補生成」と「近似将来価値による状態評価」に寄せている。AHC058 は 1 ターンごとの局所判断が後の生産連鎖に大きく効くため、貪欲 1 本より複数候補を残す価値がある。一方で全展開は重すぎるので、候補生成段階でコストを差し引いた利益が正のものだけに絞る。この絞り込みが実装量と計算時間のバランスを取っている。

## 真似するならまず実装する部分

まず `State`、`simulate_production`、親参照による出力復元を作る。次に、全候補展開ではなく `GetPromisingActions` 型の上位候補抽出を作り、ビーム幅を小さめにして正しく動かす。高速化は `powerCache`、終盤ビーム縮小、時間監視の順に足すとよい。

## 注意点・未理解点

- 近似評価は厳密な最終りんご数ではないため、候補が偏る可能性がある。
- `BigInteger` を `double` に変換して評価しているため、非常に大きい値では丸めや無限大の扱いに注意が必要である。
- 候補生成では `P=0` を 1 とみなすが、状態評価では 0 をそのまま使う。この差は未起動系列を候補に残すための工夫だが、評価の整合性には注意が必要である。
- 記事コードには状態ハッシュによる重複排除がなく、同じ `P` に至る状態が重複してビームを消費する可能性がある。
- `LEVEL_WEIGHTS` や `COST_PENALTY` の調整依存が大きい。
