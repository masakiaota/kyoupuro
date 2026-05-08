# AHC016 - terry_u16 解法メモ

## 参照元

- 記事: [HACK TO THE FUTURE 2023 予選 (AHC016) 解説](https://www.terry-u16.net/entry/httf2023-qual)
- 著者: terry_u16
- サイト: TERRYのブログ
- 種別: 上位解説、実装公開、日記
- 成績・順位: 1,377,367,921,862点、7位、perf. 2898
- コード有無: あり。記事内のGitHub [terry-u16/ahc016](https://github.com/terry-u16/ahc016) を確認した
- コードを読めたか: 読めた。`src/main.rs`、`src/encoders/isomorphism.rs`、焼きなまし・状態・同型判定周辺を確認した
- 読めなかったもの: リポジトリ内の全実験スクリプトや全パラメータ表は精読していない。最終解に直接関係するRust実装と日記を優先して読んだ

## 解法の全体像

互いに同型でない小グラフを情報として使い、その各頂点をサイズ `K` のクリークに冗長化して送る。受信グラフでは頂点がシャッフルされているため、焼きなましで `N=nK` 頂点を `n` 個のグループへ分け、各グループを元の小グラフの1頂点とみなす。復元した `n` 頂点グラフを同型判定し、元のグラフ番号を出す。

焼きなましは複数回走らせ、混同行列を使った重み付き多数決で最終回答する。グラフ候補は同型類の代表を列挙し、グラフごとの正解しやすさを事前評価して良いものから使う。

## 主要アイデア

- `n=4,5,6` の同型でないグラフを列挙し、最大 `M=100` でも `n=6` の156種類で足りるようにする。
- ノイズ対策として、元グラフの1頂点を `K` 頂点のクリークに置き換える。
- 元グラフに辺がある2頂点間は、対応する2クリーク間を完全二部グラフにする。
- 復元は、ブロック内・ブロック間の辺ができるだけ「全てある」または「全てない」に寄るようにグループ分けを焼く。
- 近傍は2グループ間の頂点swapのみ。差分計算とbitset+popcountで高速化する。
- 5回復元し、混同行列に基づいて単純多数決より情報量のある投票をする。
- `N`、`K`、スコア係数は `(M, epsilon)` ごとに事前チューニングし、埋め込み戦略表で選ぶ。

## 最終コードの構造

Rust実装では、`Encoder` traitに対する `IsomorphismEncoder` が本体である。`main.rs` は `M, epsilon` を読み、エンコーダを作って `N` と `M` 個のグラフを出力し、100クエリに対して `decode` を呼ぶだけの薄い構造になっている。

### 状態表現

- `Graph`: 隣接行列を `Vec<Vec<bool>>` として持つ基本グラフ型。
- `BinaryGraph`: 観測グラフを辺あり `+1`、辺なし `-1` に変換したもの。
- `IsomorphismEncoder`:
  - `graphs`: 送信する同型類代表グラフ。
  - `graph_size`: 実際に送る `N = original_graph_size * redundancy`。
  - `original_graph_size`: 小グラフの頂点数 `n`。
  - `redundancy`: クリークサイズ `K`。
  - `score_coef`: クリーク内スコアの重み。
  - `confusing`: 混同行列を転置した投票用行列。
- `State`:
  - `groups`: 各小頂点グループに属する観測頂点。
  - `groups_u128`: グループ所属集合のbit表現。
  - `graph_u128`: 各観測頂点の隣接集合bit表現。
  - `self_counts`: グループ内の辺寄り度。
  - `cross_counts`: グループ間の辺寄り度。

### 観測・制約・入力の扱い

- `M, epsilon` から戦略表を引き、`n`、`K`、スコア係数を決める。
- 送信用グラフは、代表小グラフの頂点を `K` 個ずつ展開して作る。
- 観測グラフは `BinaryGraph` に変換し、焼きなましの評価を `+1/-1` の和として扱いやすくする。
- 復元後の小グラフは、VF2ベースの同型判定で候補グラフと照合する。次数列で枝刈りしてからDFSする。

### 評価関数

記事上の評価は、グループ内の辺数とグループ間の多数派度を使う。

```text
score = alpha * sum_i c_ii + sum_{i<j} max(c_ij, K^2 - c_ij)
```

実装上は `BinaryGraph` の `+1/-1` 表現に寄せ、グループ内は正の寄りだけを加点し、グループ間は絶対値を加点する。

```text
inside_score = sum_i max(self_counts[i], 0)
outside_score = sum_{i<j} abs(cross_counts[i,j])
score = score_coef * inside_score + outside_score
```

これは「同じグループ内は辺が多い」「異なるグループ間は辺が全体として多いか少ないかに偏る」という冗長化構造を評価している。

### 探索・構築・更新

- 初期解は、観測頂点をランダムシャッフルして `n` グループへ均等配分する。
- 近傍は、異なる2グループから1頂点ずつ選んでswapする。
- swap前にスコアとカウント配列を退避し、採択されなければrollbackする。
- 焼きなまし温度は時間に応じて指数的に下げる。
- `graph_u128[v] & groups_u128[g]` のpopcountで、頂点移動による各ブロックカウント変化を高速に求める。
- 各クエリで5回焼き、復元できた各候補に混同行列の列を加算して投票する。

### 操作・クエリ・出力選択

- `decode` は各試行で `restore` を呼ぶ。
- `restore` は焼きなまし後、`cross_counts` の符号から小グラフの辺あり・なしを復元する。
- 復元小グラフを各送信候補と同型判定し、一致した候補番号を返す。
- 一致候補が得られたら、混同行列に基づいて `votes[j] += confusing[i][j]` と加算し、最大票の添字を出力する。

### 時間配分・パラメータ

- 各クエリの持ち時間を残り5秒から概算し、5試行へ分割する。
- `TRIAL_COUNT = 5`。
- `score_coef`、`n`、`redundancy` は巨大な戦略表から読む。
- bit表現は `N <= 100` に対して `u128` を使う。

## 実装上重要な断片

```text
encode(original_graph):
    for each original vertex i:
        make clique of K vertices
    for each original edge (i, j):
        connect all K*K pairs between group i and group j
```

```text
anneal_state(H):
    state = random_partition(N vertices into n groups)
    while time remains:
        move = swap one vertex between two groups
        apply delta to self_counts/cross_counts
        if score improves or exp(diff/temp) accepted:
            keep
        else:
            rollback counts and swap
    return best_state
```

```text
restore_graph(state):
    for each group pair (i, j):
        if cross_counts[i,j] > 0:
            edge(i, j) = 1
        else:
            edge(i, j) = 0
    return small_graph
```

## この解法の本質

頂点シャッフルで失われる番号を直接復元するのではなく、「同じ役割の頂点を束ねたクリーク」を作り、その束を復元対象にしている。これにより、本来の同型判定は小さい `n` 頂点グラフ上で行えばよくなり、ノイズ耐性は `K` 本の多数決で稼げる。

さらに、復元が失敗しやすいグラフは事前に混同行列で見えるため、単純な候補選択ではなく「どの誤り方をするか」まで出力選択に使っている。

## 真似するならまず実装する部分

最初は `n=4` または `n=5` に固定し、同型類代表の小グラフ列挙、クリーク冗長化、ランダム初期解からのswap焼きなましを作るのがよい。混同行列や戦略表は後回しでよい。

次に、`u128` bitsetとpopcountによる差分更新を入れると、試行回数を増やせて性能が伸びる。

## 注意点・未理解点

- 焼きなましは初期解依存が大きい。1回だけだと復元が偏る。
- グループ内とグループ間の重み `score_coef` は実験依存で、理論的に一意ではない。
- 混同行列は強いが、事前シミュレーションと提出コードへの埋め込みが必要で手間が大きい。
- VF2は実装されているが、記事脚注では小さい `n` なら順列全探索でも足りる可能性があるとされている。
- コード中の巨大な戦略表・精度行列は、生成過程を完全には追っていない。
