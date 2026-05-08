# AHC017 - msmits 解法メモ

## 参照元

- 記事: 16th Place: Post mortem
- URL: https://atcoder.jp/contests/ahc017/editorial/5689
- 著者: msmits
- サイト: AtCoder Editorial
- 種別: 上位解説、参加後メモ
- 成績・順位: 16位
- コード有無: あり。記事内には無いが、Browserで AtCoder 提出一覧 `https://atcoder.jp/contests/ahc017/submissions?f.User=msmits` から提出 `38656241` を確認した
- コードを読めたか: 読めた。提出詳細 `https://atcoder.jp/contests/ahc017/submissions/38656241` で、`Astar`、`ExcludedAstar`、`edgeRelation`、`edgeRelated`、`edgeInactive`、`ScoreAdd`、`ScoreRemove`、`Connected`、`RelatedBits`、`SA` を確認した
- 読めなかったもの: 記事本文にはコードリンクは無い。相性スコアの全係数の意図はコードからは分からない

## 解法の全体像

この解法は、最短路距離を毎回評価するのではなく、辺どうしの関係スコアを事前に集め、その関係スコアの総和を焼きなましで最小化する。考え方は「同じ日に工事すると代替経路を塞いでしまう辺ペアを避け、同じ経路上でまとめて工事した方がよい辺ペアは同日に寄せる」というものだ。

最初に、修理なしの距離を計算する。次に2から3秒ほど、ランダムな2頂点間の最短路を A* で取り、その経路上の各辺を1本ずつ除外して代替経路を求める。元の経路と代替経路の差分から、除外した辺と他の辺との相性を更新する。

残り時間では、同じ日に割り当てられた辺ペアの relation score 合計を最小化するように SA を行う。1回の遷移は1辺を別日に移す操作で、グラフが連結のままかも確認する。

## 主要アイデア

- スコアを直接最短距離で評価せず、辺ペアの相性スコアで近似する。
- ランダムな2点間最短路をサンプリングし、その経路上の辺を1本外したときの代替経路との差を見る。
- 元経路にあり代替経路にない辺は、除外した辺と同じ日に工事しても相対的に悪影響が小さいので、同日を促す負の関係にする。
- 代替経路にだけ現れる辺は、除外した辺と同日に工事すると迂回路を塞ぐので、同日を避ける正の関係にする。
- SA の評価は、同じ日にある辺ペアの relation score 合計である。
- 1辺移動の差分は、移動辺と移動元日の他辺、移動先日の他辺との関係だけ見ればよい。
- 高速化には `uint64_t[47]` の bitboard を使い、対象日と関係のある辺だけをビット演算で絞る。

## 最終コードの構造

Browserで AtCoder 提出 `38656241` を読んだ。以下は本文と提出コードから確認できた実装構造である。

### 状態表現

- `days[e]`: 辺 `e` の工事日。
- `edgeInactive[day][word]`: 日ごとの工事辺集合を `uint64_t` の bitboard で持つ。
- `edgeRelation[e*M+f]`: 辺 `e` と辺 `f` を同じ日にしたときの関係スコア。
- `edgeRelated[e][word]`: 辺 `e` と関係がある辺集合を表す bitboard。
- `repairCount[day]`: 各日の工事数。
- `Connected(v1, v2, day)`: その日の工事辺を抜いたグラフでの連結確認。
- `bestDays`, `bestScore`, `SAscore`: 最良割当と探索スコア。

### 観測・制約・入力の扱い

- 最初に修理なしの距離を計算する。
- 辺関係のサンプリングでは、ランダムな2頂点を選び A* で最短路を求める。
- 各サンプル経路上の辺について、その辺を除外した代替経路を再度 A* で求める。
- SA の遷移では、移動先日の工事本数が `K` を超えないことを確認する。
- 記事では、1辺移動ごとにグラフが連結か確認すると述べられている。

### 評価関数

評価は、同じ日に工事する辺ペアの relation score 合計である。

```text
score(assign) =
    sum over days d
    sum over unordered pairs (e, f) in day d
        relation[e][f]
```

- 正の relation は、同日に置くと悪い関係を表す。
- 負の relation は、同日に置くとむしろよい関係を表す。
- SA ではこの `score` を最小化する。

関係スコアの更新は次のように要約できる。

```text
for sampled path P:
    for removed edge r in P:
        Q = shortest path when r is removed
        for edge e in P but not Q:
            relation[r][e] += negative value
        for edge e in Q but not P:
            relation[r][e] += positive value
```

具体的な重み量や正規化は記事からは分からない。

### 探索・構築・更新

- 関係スコア収集:
  - 2から3秒、ランダム2頂点の経路サンプルを繰り返す。
  - 通常経路と、経路上の各辺を除外した代替経路を比較する。
  - 50,000から100,000程度の経路サンプル、辺関係更新は約100万回規模と説明されている。

- SA:
  - ランダムに1辺を選び、別日に移す。
  - 移動先日の容量を確認する。
  - その日のグラフが連結であるか確認する。
  - relation score の差分を計算し、SA の採択判定を行う。

- 差分更新:
  - 移動元日から辺 `e` を外すと、`e` と移動元日の他辺とのペアスコアが消える。
  - 移動先日に辺 `e` を加えると、`e` と移動先日の他辺とのペアスコアが増える。
  - bitboard で「同じ日にある辺」かつ「`e` と関係がある辺」を絞り、立っているビットだけを `ctz` で列挙する。

### 操作・クエリ・出力選択

- 最終出力は各辺の工事日である。
- SA では現在割り当てと日別 bitboard を常に同期させる。
- 採択時だけ、`assign`、日別集合、総 relation score を更新する。

### 時間配分・パラメータ

- 最初の2から3秒を辺関係の収集に使う。
- 残り時間で SA を行う。
- SA の反復は、おそらく 50万から100万回程度と書かれている。
- bitboard は `uint64_t[47]`。47個で 3000 辺以上を表せる。

## 実装上重要な断片

```text
build_relations:
    while time < relation_time:
        s, t = random vertices
        path = astar_shortest_path(s, t)
        for removed_edge in path:
            alt = astar_shortest_path(s, t, banned=removed_edge)
            update relation[removed_edge][*] from path vs alt
```

```text
move_delta(edge, old_day, new_day):
    delta = 0
    old_related = relation_bits[edge] & day_bits[old_day]
    for f in bits(old_related):
        delta -= relation[edge][f]

    new_related = relation_bits[edge] & day_bits[new_day]
    for f in bits(new_related):
        delta += relation[edge][f]
    return delta
```

## この解法の本質

この解法の本質は、最短路評価を毎回行わず、事前サンプリングで「どの辺とどの辺を同時に止めると悪いか」を学習する点である。AHC017 では、悪い日とは単に重要な辺が多い日ではなく、代替経路まで同時に塞いでしまう日である。元経路と代替経路の差分を見ることで、その関係を辺ペアスコアに圧縮している。

距離評価に比べると粗いが、SA の1反復が非常に軽くなる。相性スコアを bitboard で差分計算できるため、探索回数を稼げるのが強みである。

## 真似するならまず実装する部分

まずは、最短路サンプリングから relation score を作り、同日辺ペアの合計を評価する部分を作るのがよい。A* が難しければ、最初は Dijkstra で通常経路と代替経路を求めてもよい。

次に、1辺移動の SA と relation score の差分更新を作る。bitboard 最適化は後からでよいが、最初から relation を疎なリストで持つと移行しやすい。

## 注意点・未理解点

- 提出コードで A*、代替経路、bitboard 差分、連結確認は確認できた。`DIST_EDGE_POWER`、`PATH_POWER`、`TEMP_FACTOR` などの係数の由来は本文だけでは分からない。
- bitboard 差分は高速だが、relation の格納形式を間違えるとメモリまたは更新時間が厳しくなる。
- 関係スコアは近似なので、実際の距離スコアとずれる可能性がある。
- 容量制約と連結性制約を SA の遷移内で確実に守る必要がある。
