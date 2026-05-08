# AHC028 - scat_neko_ortools 解法メモ

## 参照元

- 記事: [OR-Toolsを使ってTSPの近似解を求める(Python)](https://atcoder.jp/contests/ahc028/editorial/9082)
- 著者: scat_neko
- サイト: AtCoder ユーザ解説
- 種別: 実装解説・ユーザ解説
- 成績・順位: 記事中に順位やスコアの記載なし
- コード有無: 部分コードあり。OR-Tools のTSP部分のコード片が掲載されている
- コードを読めたか: 読めた。リンク先の [OR-Tools TSP ドキュメント](https://developers.google.com/optimization/routing/tsp?hl=ja) も確認した
- 読めなかったもの: 完整な提出コードや座標列復元コードは掲載されていない

## 解法の全体像

各単語を1つのノードと見なし、単語間の移動コスト行列を作ってTSPとして解く方針である。単語ごとに、開始位置や終了位置を固定せず、その単語5文字を打つ最小コストと経路をDPで求める。そこで決まった開始・終了位置を使い、単語 `i` の後に単語 `j` を打つときの移動コストを計算する。

その後、OR-Tools の `RoutingModel` に単語間コスト行列を渡し、TSPの近似解として単語順を得る。最後に、その順序に従って各単語の経路を出力する想定である。

## 主要アイデア

- 単語順をTSPとして扱う
- 各単語を単体で打つ最短経路をDPで求め、単語の開始位置と終了位置を決める
- 単語間コストを距離行列 `cost_between_words` として作る
- OR-Tools のルーティングソルバーを利用し、200ノード程度のTSP近似解を短時間で得る
- 初期解戦略に `PATH_CHEAPEST_ARC`、局所探索メタヒューリスティックに `GUIDED_LOCAL_SEARCH` を使う
- PyPyではOR-Toolsが使えないため、CPythonを使う

## 最終コードの構造

記事にはTSP部分のみが掲載されている。以下は記事本文とコード片から読める構造である。

### 状態表現

- 各単語をTSPのノードとして扱う
- `cost_between_words[i][j]` は、単語 `i` の終了位置から単語 `j` の開始位置へ移るコストを表す
- OR-Tools側では `distance_matrix`、`num_vehicles=1`、`depot=0` を持つデータモデルを作る
- `route_global` はTSPで得た訪問順で、ダミーのdepotを除いて単語番号へ戻す

### 観測・制約・入力の扱い

- 単語ごとの最小経路はDPで計算できるとされる
- 各単語の開始位置・終了位置は、単語単体の最短経路で決定済みとする
- OR-Tools の距離行列には整数コストを渡す
- 外部ライブラリが必要なため、AtCoder環境ではCPythonで実行する

### 評価関数

- TSPの目的関数は、`cost_between_words` に沿った巡回または経路の総距離
- AHC028の本来の総コストは、単語内部の打鍵コストと単語間移動コストの和
- 記事のTSP部分は単語間順序の最適化を担当し、単語内部コストは前処理で固定されている

### 探索・構築・更新

- まず各単語の単体最短経路をDPで求める
- 次に全単語対の接続コストを計算し、`cost_between_words` を作る
- OR-Tools の `RoutingIndexManager` と `RoutingModel` を作る
- 距離コールバックで、OR-Tools内部インデックスをノード番号へ戻し、距離行列の値を返す
- `PATH_CHEAPEST_ARC` で初期解を作り、`GUIDED_LOCAL_SEARCH` で局所探索する
- 時間制限はコード片では1秒

### 操作・クエリ・出力選択

- `routing.SolveWithParameters` が解を返したら、`routing.NextVar(index)` を辿って訪問順を復元する
- 記事中のコードでは `route_global = route_global[1:]` とし、さらに `r-1` して単語番号へ戻している
- 完全な座標出力は掲載されていないが、得た単語順に対応する単語単体の経路を並べる構造だと考えられる

### 時間配分・パラメータ

- OR-Tools の探索時間は1秒
- `num_vehicles=1`
- `depot=0`
- 初期解戦略は `PATH_CHEAPEST_ARC`
- 局所探索メタヒューリスティックは `GUIDED_LOCAL_SEARCH`
- ログ出力は無効

## 実装上重要な断片

OR-Toolsへ渡す構造:

```text
data["distance_matrix"] = cost_between_words
data["num_vehicles"] = 1
data["depot"] = 0
```

距離コールバック:

```text
distance_callback(from_index, to_index):
    from_node = manager.IndexToNode(from_index)
    to_node = manager.IndexToNode(to_index)
    return cost_between_words[from_node][to_node]
```

順序復元:

```text
index = routing.Start(0)
while not routing.IsEnd(index):
    route.append(manager.IndexToNode(index))
    index = solution.Value(routing.NextVar(index))
drop depot and convert dummy-offset indices to word ids
```

## この解法の本質

この解法は、AHC特有の探索器を自作せず、単語順序の部分を既存のTSPソルバーへ委ねる点が本質である。200ノード程度ならOR-Toolsの局所探索が制限時間内にそれなりの経路を作れるため、短時間で実装できる。

一方で、単語ごとの開始位置・終了位置を単体最短で固定してしまうため、前後関係に応じて終端位置を変える上位解法の自由度は失われる。また、記事本文だけを見る限り、単語間の文字重なりを深く使う構造ではない。

## 真似するならまず実装する部分

まず、各単語を単体で打つ最短経路DPを作り、開始位置・終了位置・内部経路を保存する。次に、単語間コスト行列を作ってOR-ToolsのTSPサンプルに流し込む。

競技用に強くするより、短時間でまともな順序を得るためのベースラインとして実装するのが向いている。

## 注意点・未理解点

- 完整な提出コードは掲載されていないため、単語単体DPや最終座標列出力の詳細は不明である
- OR-Tools はPyPyで使えないため、提出言語をCPythonにする必要がある
- 単語の開始・終了位置を固定すると、隣接単語に応じた終端位置最適化ができない
- TSPは巡回問題として扱われるため、AHC028の「開始位置固定・終点自由」の形へ合わせるためにdepotやダミーノードの設計が必要である
- 外部ライブラリ依存のため、提出環境で利用可能かを確認する必要がある
