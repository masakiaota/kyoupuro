# AHC017 - ymatsux 解法メモ

## 参照元

- 記事: THIRD プログラミングコンテスト (AHC 017): 20位以内のスコアを出すシンプルな解法
- URL: https://note.com/ymatsux/n/n74ced714fe20
- 著者: ymatsux
- サイト: note
- 種別: 詳細解説、提出コード付き
- 成績・順位: 本番7位。記事で解説されているシンプル版は20位以内相当、提出コードの表示スコアは 449,093,516
- コード有無: 記事内に AtCoder 提出コードあり
- コードを読めたか: 読めた。https://atcoder.jp/contests/ahc017/submissions/38713543 を確認した
- 読めなかったもの: 7位解法の最終版コードは記事中にリンクがなく、方針概要のみ読めた

## 解法の全体像

記事の主対象は、20位以内相当を出すシンプルな焼きなましである。初期解は辺番号を日数で割った余りのような単純割り当てにし、ランダムに1辺を選んで別の日へ移す。移動元と移動先の2日だけ評価し、代表点から全頂点への最短距離和が悪化するかどうかを SA で判定する。

高速化の核は、代表点近似と最短路木を使った逐次更新である。辺が追加される場合は、その辺で短くなる端点から緩和する。辺が削除される場合は、削除前の最短路木でその辺を使っている部分木を無効化し、境界から Dijkstra 的に再確定する。

本番7位解法では、これに加えて「辺ペアの相性」だけを使う低精度だが高速な焼きなましを初期解として使ったと述べられている。

## 主要アイデア

- 全点対最短路ではなく、代表点から全頂点への距離平均で評価する。
- 代表点は格子中心に近い頂点から選ぶ。シンプル版では 5x5 格子のうち円内にある点を使い、21点になる。
- 各日、各代表点について、距離配列と最短路木の親辺を持つ。
- 1辺移動で評価が変わるのは移動元日と移動先日の2日だけである。
- 辺追加は端点緩和から始められるため比較的簡単である。
- 辺削除は、その辺が最短路木に含まれる場合だけ、子孫部分木を無効化して再探索する。
- SA の採択確率は、推定距離和の増加量 `delta` と温度で決める。
- 7位版では、二辺を同日に工事した場合の相性を別途推定し、その相性スコアを最適化する。

## 最終コードの構造

### 状態表現

- `edge_assignment[edge]`: 各辺の工事日。
- `day_to_is_edge_active[day][edge]`: 日ごとの辺が通行可能かどうか。工事中なら `0`。
- `day_to_assignment_count[day]`: 日ごとの工事本数。
- `day_sample_to_distance_vector[day][sample][v]`: 日 `day`、代表点 `sample` から頂点 `v` への距離。
- `day_sample_to_parent_edge_vector[day][sample][v]`: 最短路木で頂点 `v` へ入る親辺と親頂点。
- `sample_vs`: 代表点の頂点番号。
- `sample_weights`: 代表点ごとの重み。全頂点を最も近い代表点へ割り当て、その比率を重みとする。
- `history_vector`: 差分更新で変更した距離と親辺を戻すための履歴。

### 観測・制約・入力の扱い

- 初期割り当ては `edge_id % D`。
- `day_to_is_edge_active` は全辺 active から始め、工事日に対応する辺を inactive にする。
- 移動先日の工事本数が `K` 以上なら遷移を試さない。
- 代表点は `MakeSampleVertexes(5)` で作る。5x5 格子の中心のうち円内の点に最も近い頂点を選ぶ。
- 代表点重みは、各頂点が最も近い代表点に属するとみなし、その所属数を `N` で割って作る。

### 評価関数

1辺を `day_0` から `day_1` へ移すとき、推定スコア差は次の2つの和である。

```text
delta =
    old_day に辺を戻したことによる距離和変化
  + new_day から辺を削除したことによる距離和変化
```

各代表点の変化量は `sample_weight / D` を掛けて足す。`delta <= 0` なら改善であり、`delta > 0` でも `exp(-delta / temperature)` で採択する。

温度は指数補間で、提出コードでは `10.0` から `0.1` へ下げている。

### 探索・構築・更新

- 初期化:
  - 全頂点からの元グラフ最短距離を計算して `base_distance_matrix` を作る。
  - 初期割り当てから、各日の active edge 配列を作る。
  - 各日、各代表点に対して通常の Dijkstra を実行し、距離と親辺を保存する。

- SA ループ:
  - ランダムに辺 `edge_id` を選ぶ。
  - 現在の日 `day_0` とランダムな移動先 `day_1` を選ぶ。
  - 同じ日、または移動先が満杯ならスキップする。
  - `day_0` では工事をキャンセルするので edge active にする。
  - `day_1` では工事を追加するので edge inactive にする。
  - 代表点ごとに、辺追加と辺削除の逐次更新を行い、`delta` を得る。
  - 採択なら割り当てと日別工事数を更新し、履歴を破棄する。
  - 棄却なら active 配列と距離、親辺を履歴から巻き戻す。

- 辺追加の逐次更新:
  - 追加辺の片側経由で反対側が短くならないなら変化なし。
  - 短くなる端点を priority queue に入れ、通常の Dijkstra と同様に短縮を伝播する。

- 辺削除の逐次更新:
  - 削除辺が親辺として使われていないなら変化なし。
  - 使われている場合、親子関係を辿って子孫部分木を invalidated にする。
  - invalidated 頂点のうち、外側から到達できる頂点を初期候補として queue に入れる。
  - queue から Dijkstra 的に距離を確定し、まだ到達できない頂点は `INF` にする。

### 操作・クエリ・出力選択

- 出力は `edge_assignment[edge] + 1`。
- 探索中、`day_to_is_edge_active` と `edge_assignment` の対応を保つ。
- 棄却時は履歴を逆順に辿り、距離と親辺を復元する。

### 時間配分・パラメータ

- SA 終了時刻は `5700ms`。
- 温度は `MAX_TEMPERATURE = 10.0` から `MIN_TEMPERATURE = 0.1`。
- 代表点は `MakeSampleVertexes(5)` により、円内の 5x5 格子中心に対応する点を使う。記事では21点として説明されている。
- seed 0 の実験として、代表点4点では約8000回、逐次更新導入後は約120万回、21点でも20万回以上の試行回数が出たと説明されている。

## 実装上重要な断片

```text
try_move(edge):
    old_day = assignment[edge]
    new_day = random_day()
    if new_day is full:
        continue

    activate edge on old_day
    deactivate edge on new_day

    delta = 0
    for sample in samples:
        delta += add_edge_update(old_day, sample, edge) * weight(sample) / D
        delta += delete_edge_update(new_day, sample, edge) * weight(sample) / D

    if exp(-delta / temperature) >= random():
        commit assignment and clear histories
    else:
        restore active flags
        rollback distance and parent histories
```

```text
delete_edge_update(root, edge):
    if edge is not parent of either endpoint:
        return 0
    r = endpoint whose parent is the other endpoint
    invalidated = subtree rooted at r in shortest path tree
    for v in invalidated:
        find best edge from non-invalidated neighbor
        if found:
            push v into priority queue
    run dijkstra from boundary candidates
    remaining invalidated vertices become INF
```

## この解法の本質

この解法の本質は、AHC017 の最小実装として必要な要素をかなり素直に切り出している点である。強い初期解や複雑な辺順がなくても、代表点近似と最短路木の差分更新があれば、1辺移動 SA だけで十分な探索回数を稼げる。

特に、辺削除時の「最短路木の子孫を無効化し、境界から再探索する」処理は、この問題の中心部である。全頂点を再計算しないため、1回の遷移が軽くなり、焼きなましが現実的になる。

7位版の相性初期解は、さらに評価を「辺ペアの相性」に落とす方向である。距離評価より粗いが非常に速く、良い初期解を作ってから距離ベース SA で仕上げる構成と読める。

## 真似するならまず実装する部分

この参照元は、理解用の最小実装として最も真似しやすい。まずは次を作るのがよい。

- `edge_assignment` と `day_to_is_edge_active`。
- 代表点を 2x2 または 5x5 格子から選ぶ処理。
- 各日、各代表点の Dijkstra と親辺保存。
- 1辺移動 SA。
- 辺追加・辺削除の履歴付き差分更新。

最初から7位版の相性初期解を作る必要はない。まずシンプル版を動かし、差分更新と rollback が正しいことを確認するべきである。

## 注意点・未理解点

- 7位解法の低精度 SA の詳細コードは読めなかった。記事にある相性の定義と省略方針だけを確認した。
- 親辺を1本だけ持つため、同じ距離の最短経路が複数ある場合の扱いは慎重にテストする必要がある。
- `history_vector` は差分更新中に何度も同じ頂点を更新し得るので、復元順序を逆順にする必要がある。
- 到達不能になった頂点を `INF` にする処理を忘れると、非連結状態を過小評価する。
- 代表点重み付き評価は全点対距離そのものではない。代表点の選び方でスコアが大きく変わる可能性がある。
