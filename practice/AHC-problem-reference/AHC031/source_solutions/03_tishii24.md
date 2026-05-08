# AHC031 - tishii24 解法メモ

## 参照元

- 記事: AHC031 7位解法
- URL: https://zenn.dev/tishii2479/articles/7e9f891fb93e46
- 著者: tishii24
- サイト: Zenn
- 種別: 上位解説、公開コード
- 成績・順位: 7位
- コード有無: 記事内に AtCoder 提出リンクと GitHub リポジトリあり
- コードを読めたか: 読めた。AtCoder 提出 `51939931`、GitHub `tishii2479/ahc031` の README、`src/solver.rs`、`src/def.rs`、`sub.rs` を確認した
- 読めなかったもの: なし

## 解法の全体像

幅方向を複数の列に分け、各列の中で予約を縦に積む。列幅を固定した短冊型だが、日ごと・列ごとの予約順序や列割り当てを焼きなましで変える。評価は、2日分を同時に見て、前日から当日、当日から翌日の縦境界がどれだけ再利用できるかを測る。

特徴的なのは、単に高さ列を比較するのではなく、前日までの「余り」をグラフとして伝播し、当日の境界位置へ余りを使って合わせられるかを判定する点である。これにより、空き領域を境界一致のためのバッファとして扱える。

## 主要アイデア

- 列幅 `ws` を固定し、各予約はどれか1列に入れる。
- 各列の予約高さは `ceil(area / width)` で決める。
- 高さ合計が `W` を超える列は、余白が小さい予約を優先的に削って面積不足コストを計算する。
- 仕切り変更は、列ごとに隣接日の高さ列をマッチングして近似・評価する。
- `match_greedy` は、前日の高さ列、前日から引き継げる余り、次の日の高さ列を走査し、境界を合わせられるグループ数を数える。
- `ColGraph` で過去の余りを管理し、当日に使える余白を次の日以降へ伝える。
- 焼きなましでは、列内 swap、列間1個移動、列間の連続ブロック交換を使う。
- `d` 日目と `d+1` 日目を同時に最適化し、局所的に未来を見た評価にする。

## 最終コードの構造

### 状態表現

主な状態は `State` にまとまっている。

- `ws`: 列幅。
- `r[d][col]`: `d` 日目の `col` 列に入る予約 index の列。
- `heights[d][col]`: 各予約を列幅 `ws[col]` で置いたときの高さ。
- `height_sum[d][col]`: 列ごとの高さ合計。
- `squeezed_heights[d][col]`: 高さ合計が `W` を超えた場合に、削った後の高さ列。
- `exceed_cost[d][col]`: 削ったことで発生する面積不足コスト。
- `prev_h[col]`: 前日までに確定した列内高さ列。
- `prev_rem[col]`: 前日以前から使える余り。
- `graphs[col]`: 余りの伝播を保持する列ごとのグラフ。
- `score_col[col]`: 列ごとの仕切りコスト相当と面積不足コスト。

`need_update_squeezed` により、列内の高さ列を変更したときだけ `squeezed_heights` を再計算する。

### 観測・制約・入力の扱い

- 入力面積 `A[d][k]` はそのまま列幅で割り、高さ `ceil(A[d][k] / ws[col])` に変換する。
- 列の高さ合計が `W` 以下なら面積不足なし。
- 高さ合計が `W` を超える場合、`to_squeezed_height` で高さを削り、削った面積を不足コストとして加算する。
- 列幅固定なので、同じ列内の長方形は縦に積めば非重複になる。
- 出力時は列の x 範囲と累積 y 座標から長方形を復元する。

### 評価関数

列ごとの評価は `(switch_cost, exceed_cost)` の形で持つ。

```text
score_col =
    switch_cost_between(prev, day)
  + switch_cost_between(day, next_day)
  + exceed_cost(day)
  + exceed_cost(next_day)
```

`eval_col(d, col)` は、`d-1 -> d` と `d -> d+1` の境界一致を同時に見ている。日によって重みが変わり、初日は `L_0=0` なので前日側の重みを 0 にする。中間日は未来側を軽めにし、最後に近い日は両側を重くする設計である。

境界一致は、単純な累積高さの一致だけでなく、前日から使える余りと当日の余りを使って合わせられるかを `match_greedy` で判定する。

### 探索・構築・更新

`solve` は次の流れで進む。

```text
setup_heights()
for d in 0..D:
    setup_prev_h_rem(d)
    if d != D - 1:
        optimize_r(d)
    total_cost += to_next_d(d)
create_answer(total_cost)
```

`optimize_r(d)` は `d` 日目と `d+1` 日目の予約割り当てを焼きなましで改善する。

近傍は3種類である。

- 列内で 1 から数回の swap。
- 列間で予約を1個移動。
- 2列間で連続ブロックを交換する `n:n` swap。

各近傍では、変更した列だけを `eval_col` で再評価し、`score_col` の差分で採択判定する。悪化許容は温度と乱数から作る閾値 `-T * log(rand)` による。

### 操作・クエリ・出力選択

- `to_next_d(d)` で `d` 日目の配置を確定し、次の日へ `ColGraph` の状態を進める。
- `match_greedy` のグループ情報を使い、どの前日ノードと当日高さ列が接続されるかを決める。
- `create_answer` で、列幅と高さ列から各予約の `(i, j, i', j')` を復元する。

### 時間配分・パラメータ

- 時間制限は `TIME_LIMIT = 2.95`。
- 各日への探索時間は、残り時間を残り日数で割って決め、最後付近は少し厚くする。
- 焼きなまし温度、`d` 日と `d+1` 日を選ぶ比率、初期候補数が `Param` にある。
- 記事では、焼きなましの繰り返しだけでなく、列幅候補を複数試してよい初期構造を選ぶことが重要と説明している。

## 実装上重要な断片

```text
to_squeezed_height(col):
    over = height_sum - W
    while over > 0:
        choose rectangle with small lost_area_per_height
        reduce its height
        exceed_cost += lost_area
```

```text
match_greedy(prev_h, prev_rem, next_h):
    scan cumulative heights of prev and next
    collect usable prev remainder
    if one side can be raised by available remainder:
        fix a matched group
        consume remainder
    return matched_group_count
```

```text
anneal_move():
    choose d or d+1
    apply one neighborhood
    new_score = eval only affected columns
    if score_diff <= threshold:
        commit score_col
    else:
        rollback move
```

## この解法の本質

空き領域を単なる未使用スペースとして捨てず、翌日以降の境界合わせに使える「余り」としてモデル化している点が本質である。短冊型で配置を単純化しながら、日間の対応関係を `ColGraph` と greedy matching でかなり細かく評価している。

また、1日ずつ確定しながらも `d` と `d+1` を同時に最適化するため、完全な逐次貪欲より未来の仕切り変更を抑えられる。実装難度は高いが、評価関数が問題の仕切りコストへかなり近い。

## 真似するならまず実装する部分

最初は `ColGraph` を省略し、列幅固定、列内縦積み、列間移動と列内 swap の焼きなましを実装するのがよい。評価は「高さ合計超過ペナルティ」と「隣接日の累積高さ一致数」だけで始める。そこから、余りを使った `match_greedy` を追加すると、この解法の本質に近づく。

## 注意点・未理解点

- `match_greedy` と `ColGraph` の余り伝播は強力だが、実装を少し間違えると実スコアと評価がずれる。
- 高さ超過時の `squeezed_heights` は、面積不足を最小化する局所処理であり、全体最適ではない。
- `eval_col` の重みは実スコアそのものではなく探索用の近似を含む。
- 列幅候補や温度など、パラメータ依存の部分が大きい。
- 逐次確定型なので、かなり後の日の都合までは直接見ない。
