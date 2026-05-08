# AHC061 - mya3_official 解法メモ

## 参照元

- 記事: https://atcoder.jp/contests/ahc061/editorial/16465?lang=ja
- 著者: mya3
- サイト: AtCoder
- 種別: 公式解説、詳細解説
- 成績・順位: Perf. 2325 と解説タイトルに記載
- コード有無: 記事本文には最終コードリンクなし。Browser調査済みキャッシュとして AtCoder AC 提出 `#73569680` のC++実装を読めた
- コードを読めたか: 読めた。`/tmp/ahc_impl_cache/ahc061_04_mya3_mya3_73569680.cpp`
- 読めなかったもの: 記事本文だけでは、具体的な評価関数の重みや完全な状態更新コードは公開されていない

## 解法の全体像

AIの内部パラメータをEMアルゴリズムで推定し、その推定パラメータを使ってAIの行動をシミュレートする。自分の手はUCB1を用いたMCTSで決める。探索木は各ターンの現在状態を根として作り直し、自分の手だけを分岐として持つ。ロールアウトは最大12ターン先まで進め、終端局面を複数要素の重み付き評価関数で評価し、根の子のうち訪問回数が最大の手を出力する。

状態管理は高速化を重視し、01で表せる盤面情報を `__uint128_t` の下位100bitで持つ。連結成分や到達可能マスはビットシフトBFSで更新する。推定、探索、プレイアウトを2秒内で回すために、状態更新と候補列挙の定数倍をかなり削っている。

## 主要アイデア

- 盤面の集合情報を100bitのビットボードで管理する。
- 各プレイヤーについて所有マス、現在地と連結する自陣、到達可能マスを持つ。
- AIの行動を「ランダム選択」と「温度付きsoftmaxで近似した貪欲選択」の混合モデルとして扱う。
- EMで、各観測手がランダム由来だった確率と、重みパラメータ `wa..wd`、`eps` を更新する。
- `wa..wd` のM-stepはAdamで最適化する。
- MCTSは自分の手だけをノード化し、AIは推定パラメータに従ってシミュレートする。
- ロールアウト中の自分の手は、価値あるマス `b_worth` の外周寄りから確率的に選ぶ。
- 最終手は根の子の訪問回数最大、同数なら評価値合計最大で決める。

## 最終コードの構造

以下は記事本文とキャッシュ実装 `#73569680` から確認した構造である。

### 状態表現

- 01情報は `__uint128_t` の下位100bitで持つ。
- レベルと所有者は `array<uint8_t>` の4bitずつで管理する。
- 盤面全体で `b_max`（レベル最大マス）と `b_exist`（プレイヤー駒が存在するマス）を持つ。
- 各プレイヤーで、現在地、スコア、`b_owned`、`b_connected`、`b_reachable` を持つ。
- 選択可能マスは `b_reachable & ~b_exist`、価値ある選択肢はそこから `b_max` などを除いた `b_worth` として扱う。

### 観測・制約・入力の扱い

- `b_connected` は現在地ビットから始め、上下左右シフトと `& b_owned` を変化がなくなるまで繰り返して求める。
- 自陣にマスを加えた場合、隣接する未連結所有マスがあれば、`b_connected | b_pos` からBFSをやり直す。
- 自陣からマスが削除された場合、8近傍の連結状態が変わるかを前計算テーブルで判定し、必要なプレイヤーだけ現在地からBFSを行う。
- 8近傍パターンは `2^8` 通りを前計算し、各セルのシフト数とマスクも埋め込む。`_pext_u32` でもできるが、実装変更コストで見送ったとある。
- AI観測は、各ターンを「合法手内でのカテゴリごとの最大価値/1000」「選択カテゴリ」「カテゴリ内最大価値の場所が選ばれたか」「合法手数」に変換して履歴に持つ。

### 評価関数

- EMでは、AIの貪欲選択確率を温度付きsoftmaxで近似する。
- E-stepでは、観測手がランダム由来である確率 `gamma_t` をベイズ更新で計算する。カテゴリ内最大価値でない手なら貪欲では選ばれないため `gamma_t=1`。
- M-stepの `eps` は `gamma_t` の平均で更新する。
- M-stepの `w` は、`(1-gamma_t) * log p0_t` の和を最大化する。勾配は「実際に選ばれたカテゴリ」と「softmax予測」の差に価値を掛けた形になる。
- MCTSのロールアウト評価は、AIトップとのスコア比の対数、自陣内外の境界長、AI同士の選択衝突確率、AIトップ陣地と自陣の距離などを重み付きで足す。

### 探索・構築・更新

- 各ターン、現在状態を根としてMCTS木を新規構築する。前ターン木の引き継ぎは試したが下がったため不採用。
- ノードは自分の手だけで分岐し、基本的に `b_worth` のみを対象にする。
- 1回の探索は Select、Expand、Rollout、Backpropagate の流れ。
- SelectはUCB1で子を選ぶ。
- Rolloutは根から12ターン経過するか終了まで進める。
- AIは推定パラメータに基づき、問題設定と同じランダム・貪欲混合で選ぶ。
- AIのランダム行動では、ビット集合からのランダム抽出に `_pdep_u64` を使う設計がある。フォールバックも用意したと書かれている。
- ローカル環境では各ターン約2000回探索できたとある。

### 操作・クエリ・出力選択

- ロールアウト中の自分の手はランダムに選ぶ。
- 選択分布は、50%で `b_worth` の外周、35%で外周+ひと回り内側、15%で `b_worth` 全体から選ぶ。候補がなければ下位の広い集合へ移る。
- 根での最終決定は、訪問回数が最も多い子を選ぶ。同数なら評価値合計が大きい子を選ぶ。

### 時間配分・パラメータ

- 1観測ごとにEMを20回繰り返す。
- 実装の手選択は `DecideMyChosenPosWithMCTS(game.state, 20000, 12, 1.0, true, 19800000)`。
- MCTSのrollout深さは12、UCB定数は1.0。
- 自分の探索ノードは価値ある候補だけに絞る `is_only_worth=true`。

## 実装上重要な断片

キャッシュ実装では、`Solver::Solve` が毎ターンMCTSで自分の手を決め、ターン結果を読んだあと各AIモデルを `Update(game, pi, observed, 20)` で更新する。`Judge::InputTurnResult` はAIの選択位置を読むが、盤面更新はローカルの `game.ApplyActions(res)` で決定的に再構築している。

MCTS木は自分の手だけを分岐し、AIはEM推定パラメータに基づいてランダム/貪欲混合で動かす。rollout中の自分の手は、50%で価値候補の外周、70%で前線、残りで任意候補を選ぶような確率的方策である。

終端評価 `MCTSEvaluate` は、時間に応じた `log2(p0.score)-log2(best_ai.score)` に、序盤用のスコア差、自陣数、frontier、敵frontier圧、到達可能な敵領土、相手の到達/衝突確率、トップAI領土への距離ペナルティを混ぜる。

```text
update_estimator(observation):
    history.append(summary)
    repeat 20 times:
        for each history t:
            if chosen action cannot be greedy:
                gamma[t] = 1
            else:
                p_random = 1 / legal_count[t]
                p_greedy = softmax_prob(category_values[t], w, beta)
                gamma[t] = eps*p_random / ((1-eps)*p_greedy + eps*p_random)
        eps = average(gamma)
        grad_w = weighted_average((chosen_onehot - softmax_prob) * category_value)
        w = adam_update(w, grad_w)
        clamp parameters to generation range
```

```text
mcts(root_state):
    while time remains:
        path = select_by_ucb1(root)
        child = expand_one_untried_move(path.last)
        rollout_state = simulate_to_depth(child.state, depth=12)
        value = evaluate_rollout_end(rollout_state)
        add value and visit count to nodes in path
    return child_with_max_visit_count(root)
```

```text
bitboard_connected(pos, owned):
    connected = bit(pos)
    while true:
        next = connected | (shift4(connected) & owned)
        if next == connected:
            return connected
        connected = next
```

## この解法の本質

統計推定と探索をかなり正攻法に組み合わせている。AIの行動原理は明示的に与えられているため、行動履歴から隠れパラメータを推定できる。推定したパラメータで未来をサンプリングし、MCTSで自分の手を選ぶことで、局面評価の難しさを短いシミュレーションで補っている。

さらに、EMの貪欲選択を厳密なargmaxではなく温度付きsoftmaxで滑らかに近似している点が実装上強い。勾配が得られるためAdamで安定して更新でき、観測が少ない序盤でもパラメータを少しずつ寄せられる。

## 真似するならまず実装する部分

まずはビットボードでなくてもよいので、正確な状態遷移とAI行動確率を作る。次に、観測履歴をカテゴリ最大価値と選択カテゴリへ要約し、EMで `eps` と `w` を更新する部分を実装する。

MCTSは最初から複雑な評価関数を入れず、深さ8〜12のランダムロールアウト終端 `log(S0/SA)` 程度から始めるのがよい。その後、境界長やトップAI距離などを足して、ローカル比較で重みを調整する。

## 注意点・未理解点

- キャッシュ実装で、MCTS呼び出しパラメータ、rollout深さ、UCB定数、候補制限は確認できた。
- 評価関数の全重みはコード中に存在するが、本文からは読み取りにくく、再実装時はローカル調整が必要である。
- EMの履歴管理と更新回数は確認できたが、数値安定化の意図まではコードだけでは判断しにくい。
- 状態差分更新は高速化の肝だが、実装を間違えると連結成分や駒復帰で壊れやすい。
