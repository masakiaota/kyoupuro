# AHC007 - terry_u16 解法メモ

## 参照元

- 記事: THIRDプログラミングコンテスト2021（AHC007）解説 https://www.terry-u16.net/entry/ahc007-explanation
- 著者: terry_u16
- サイト: TERRYのブログ
- 種別: 上位解説、コード付き解説、提出コード
- 成績・順位: 記事中の最終提出は 14,194,012,857 点、22 位
- コード有無: あり。記事本文の C++ コード、Rust 提出 https://atcoder.jp/contests/ahc007/submissions/27880477、補正実験提出 https://atcoder.jp/contests/ahc007/submissions/27902230 / https://atcoder.jp/contests/ahc007/submissions/27924080 / https://atcoder.jp/contests/ahc007/submissions/27923984 を確認した
- コードを読めたか: 読めた
- 読めなかったもの: なし

## 解法の全体像

未判明の辺長を何通りかランダムに仮定し、現在辺を採用した場合と採用しなかった場合で、残りの将来辺から Kruskal で連結を完成させる総コストを比較する。各サンプルで「現在辺を採る世界」と「採らない世界」を作り、総コストの合計が小さい方を選ぶ。

記事のコードは C++ の教育的な実装で、ランダム距離セットを 14 個だけ作って毎ターン使う。最終提出は Rust で高速化され、サンプル数や実装細部を詰めている。さらに記事後半では、乱数生成範囲を `[d, 3d]` そのものではなく、固定値 `2d` や `1.75d` のような補正に変えるとスコアが伸びることも示している。

## 主要アイデア

- 未知の辺長を複数のサンプルで埋め、各サンプルでオフライン MST に近い判断をする。
- 現在辺を採る場合は、現在 Union-Find に現在辺を追加し、残り辺をランダム長の昇順で Kruskal する。
- 現在辺を採らない場合は、現在 Union-Find のまま残り辺を Kruskal する。
- 採用時総コスト合計 `use_cost_sum` と不採用時総コスト合計 `pass_cost_sum` を比較し、採用時の方が小さければ採る。
- すでに両端が連結済みの辺は必ず不採用にする。
- `d` から `3d` の一様分布をそのまま使うより、補正した分布・固定値を使う方がオンライン戦略の偏りに合う場合がある。

## 最終コードの構造

### 状態表現

- `input`: `n`, `m`, 座標、辺端点を持つ。
- `random_dists`: サンプル数 `count` 個の辺長配列。各配列は全 `M` 辺の仮想長を持つ。
- `union_find`: 採用済み辺の連結状態。
- `sorted_edge_indice`: あるサンプルにおいて、現在ターン以降の辺を仮想長でソートした index 列。

### 観測・制約・入力の扱い

- 初期入力から全辺のユークリッド距離を計算する。
- 各ターンで真の `edge_distance` を読む。
- 現在 Union-Find で両端が同一成分なら、その辺は MST 的に不要なので不採用。
- 各サンプルの未判明辺長は、記事基本実装では `uniform_int_distribution(d, 3d)` で生成する。

### 評価関数

各サンプルで次を計算する。

```text
use_cost  = current_length + kruskal(after current, UF + current)
pass_cost = kruskal(after current, UF)
```

全サンプルで合計し、`use_cost_sum <= pass_cost_sum` なら現在辺を採用する。`pass_cost` 側で連結不能なら大きな値を返し、採用を強く促す。

### 探索・構築・更新

- 探索は局所探索ではなく、サンプルした世界ごとの Kruskal シミュレーションである。
- サンプルごとに `edge_index + 1` 以降の辺を仮想長昇順に並べる。
- 採用ケースと不採用ケースで Union-Find をコピーし、Kruskal を走らせる。
- Kruskal は、異なる成分を結ぶ辺だけを追加し、必要な連結が完成するまでコストを積む。
- 最終提出では Rust 化やサンプル数調整で時間内の試行数を稼いでいる。

### 操作・クエリ・出力選択

- `same(u, v)` なら `0`。
- そうでなければ、サンプル平均で採用時コストが不採用時コスト以下かを見る。
- 採用なら `1` を出して Union-Find を merge、そうでなければ `0`。

### 時間配分・パラメータ

- 記事の基本 C++ 実装は `EDGE_SET_COUNT = 14`、乱数 seed は固定。
- 最終 Rust 提出はサンプル数を増やし、高速化で 22 位相当まで上げている。
- 補正実験では未知距離を固定 `2d`、固定 `1.75d` などにする提出もあり、`1.75d` の方が記事基本より高スコアだった。

## 実装上重要な断片

```text
decide(edge i, true_length):
    if uf.same(u_i, v_i):
        return false

    use_sum = 0
    pass_sum = 0
    for sample in random_dists:
        order = sort_edges_by_sample_length(i + 1, sample)

        uf_use = uf.copy()
        uf_use.merge(u_i, v_i)
        use_sum += true_length + kruskal(uf_use, order, sample)

        uf_pass = uf.copy()
        pass_sum += kruskal(uf_pass, order, sample)

    return use_sum <= pass_sum
```

## この解法の本質

オンラインで真の MST を直接知ることはできないため、未知部分を複数回サンプリングして「いま採ることの機会費用」を推定する。Kruskal を使うことで、サンプル世界での最適な残り選択をかなり素直に近似できる。実装が単純で、問題文の分布をそのまま使っても高いベースラインになる点が強い。

## 真似するならまず実装する部分

最小実装としては、記事の C++ 版どおりにサンプル数 10 から 20 程度で `use_cost` と `pass_cost` を比較する。次に、辺のソートを前計算・キャッシュし、Union-Find コピーを軽くする。スコア改善を狙うなら、サンプリング範囲や固定倍率を変えてローカルテスタで比較する。

## 注意点・未理解点

- サンプル数が少ないと判定がかなり揺れる。とはいえ毎ターン 2 回 Kruskal を走らせるので、増やしすぎると TLE する。
- `pass_cost` が連結不能になるケースを適切に大きな値で扱わないと、終盤に連結失敗しやすい。
- 分布補正が効く理由は、真の分布推定というよりオンライン戦略の過小評価・過大評価を補正しているためで、理論的に一意ではない。
- 記事本文の C++ 実装と最終 Rust 提出は同じ骨格だが、完全に同一のコードではない。
