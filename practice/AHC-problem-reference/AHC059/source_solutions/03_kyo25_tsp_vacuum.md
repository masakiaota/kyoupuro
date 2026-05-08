# AHC059 - kyo25_tsp_vacuum 解法メモ

## 参照元

- 記事: https://atcoder.jp/contests/ahc059/editorial/15045
- 著者: Kyo25
- サイト: AtCoder
- 種別: 詳細解説、コード付き
- 成績・順位: 記事内に明示なし
- コード有無: あり。本文内にC++コードが掲載されている
- コードを読めたか: 読めた
- 読めなかったもの: 提出リンクは記事内になかった

## 解法の全体像

まず、スタックをほぼ使わず「1ペアずつ連続で取る」と仮定し、各ペアを有向辺として並べるTSP風のバックボーンを作る。各ペアには2つの訪問方向があるため、状態はペア順序 `order` と向き `dirs` である。このバックボーンを焼きなましで短くする。

その後、完成したバックボーンをそのまま出力するのではなく、バックボーン上で将来訪れる予定のペアを、現在処理中のペアの内側に再帰的に割り込ませる。割り込み候補は「将来そのペアをバックボーンから消せる得」と「今寄り道して拾う損」を比べ、利益が十分なら採用する。記事ではこの部分をVacuumと呼んでいる。

## 主要アイデア

- スタック制約を直接最適化せず、まずTSPとして扱いやすい順序を作る。
- ペア `g` は `p1 -> p2` または `p2 -> p1` の有向パスとみなす。
- 初期解は盤面を4x4クラスタに分け、Z字・S字・ランダムなクラスタ順を複数試す。
- TSP焼きなましでは2-opt風reverse、短区間insert、swap、向きflipを使う。
- 後処理で、将来コスト削減量 `Gain` と現在の寄り道増分 `Detour` から `Profit = Gain - Detour` を計算する。
- 利益のある候補を再帰的に内側へ入れることで、スタックの入れ子を後付けで作る。

## 最終コードの構造

### 状態表現

- `Point { y, x }`: 盤面座標。
- `Group { p1, p2 }`: 同番号2枚の位置。
- `order[200]`: バックボーン上のペア訪問順。
- `dirs[200]`: 各ペアの訪問向き。`0` なら `p1 -> p2`、`1` なら `p2 -> p1`。
- `dist_table[400][400]`: 各ペア端点間の距離。端点は `2*g + endpoint` で表す。
- `internal_dist[g]`: ペア内距離。
- `removal_gain[g]`: バックボーンからペア `g` を取り除くと短縮できる接続距離。
- `operations`: 出力する操作列。
- `remaining_set`: まだ処理していないペア集合。Vacuum中に再帰的に削る。

### 観測・制約・入力の扱い

- 入力から各番号の2点を `groups` に格納する。
- 全ペア端点間のマンハッタン距離を事前計算する。
- `X` は使っていない。出力は移動と `Z` のみ。
- Vacuumは再帰呼び出しで `A_start, B_start, ..., B_end, A_end` の順に出力するため、スタック制約を満たす入れ子を自然に作る。

### 評価関数

焼きなまし中の評価はバックボーン距離である。

```text
backbone_cost(order, dirs):
    cost = distance((0,0), entry(order[0]))
    for each pair g in order:
        cost += internal_dist[g]
        cost += distance(exit(g), entry(next_g))
    return cost
```

Vacuumでは、候補 `B` を今入れる価値を次で評価する。

```text
base_dist = distance(current_pos, A_end)
detour = distance(current_pos, B_in) + distance(B_out, A_end) - base_dist
profit = removal_gain[B] - detour
```

コードでは `profit > -2` かつ `detour < 20` の候補から最大profitを選ぶ。

### 探索・構築・更新

初期構築:

- 各ペアの中心座標で4x4クラスタに割り当てる。
- 複数のクラスタ訪問順を作る。
- 各クラスタ内では現在位置から近い端点を持つ未使用ペアを貪欲に選び、向きも決める。
- 全候補の中でバックボーン距離が最小の初期解を採用する。

焼きなまし:

- 時間制限は約1.98秒。
- 温度は `2500.0` から `0.01` へ指数的に下げる。
- 50%でreverse、30%で短区間insert、残りでswapまたはflip。
- reverseは区間を反転し、区間内の向きも反転する。差分は区間外との接続2本だけで計算する。
- insert、swap、flipは実装を簡単にするため全体コストを再計算している。
- 採択は通常の焼きなましで、`delta < 0` または `exp(-delta/temp)`。

Vacuum:

- `removal_gain` をバックボーン上の前後接続から計算する。
- バックボーン順に未処理ペアを処理する。
- ペア `A` の開始端点を取った後、まだ未処理の候補 `B` を全探索する。
- `Profit` 条件を満たす最良候補があれば、`B` を `remaining_set` から消して再帰処理する。
- 候補がなくなったら `A` の終了端点へ移動して `Z` する。

### 操作・クエリ・出力選択

- `add_move(from, to)` で `U/D/L/R` を追加する。
- 各ペアの開始端点と終了端点で `Z` を追加する。
- `generate_output_with_gain()` の最後に操作数が16000を超えていれば切り詰める処理がある。

### 時間配分・パラメータ

- SA時間制限: `1.98` 秒。
- 温度: `TEMP_START=2500.0`, `TEMP_END=0.01`。
- reverse長: 最大30。
- insertブロック長: 最大6。
- Vacuum条件: `profit > -2`, `detour < 20`。
- 乱数: Xorshift128。
- 最大操作数: `16000`。

## 実装上重要な断片

バックボーンreverseは、区間内部のペア順と向きを同時に反転する。

```text
reverse_segment(i, j):
    old = edge(prev, entry_i) + edge(exit_j, next)
    new = edge(prev, old_exit_j) + edge(old_entry_i, next)
    if accept(new - old):
        reverse(order[i..j])
        reverse(dirs[i..j])
        for k in i..j:
            dirs[k] ^= 1
```

Vacuumの再帰は、ペアを入れ子にする操作そのものである。

```text
process(A):
    move to A_start
    Z
    while exists profitable B:
        remove B from remaining
        process(B)
    move to A_end
    Z
```

## この解法の本質

大域順序とスタック活用を分離している点が本質である。スタック制約を最初から全探索すると状態が複雑になるため、まず「1ペアずつ消す巡回路」という強い近似で全体の移動経路を作る。その後、バックボーンから削ると得になるペアを、現在の道中に吸い込む形で入れ子化する。TSPとして見える部分をTSPとして解き、スタックは局所的な利益判定で使うという割り切りである。

## 真似するならまず実装する部分

まず `order, dirs` を持つ単純な1ペア連続回収解を作り、バックボーン距離を計算する。次に2-opt reverseだけを入れた山登りまたは焼きなましを実装する。その後、`removal_gain` と `detour` による1段Vacuumを追加する。再帰Vacuumやinsert/swap/flipは、基本の出力検証ができてから足すべきである。

## 注意点・未理解点

- 記事末尾で著者自身がコードの詳細理解に不安を述べており、解説本文は生成AI由来と明記されている。
- `operations.resize(MAX_OPERATIONS)` は安全策だが、もし実際に切り詰めが発生すると全カードを消せない可能性がある。提出上問題なかったかは記事本文からは確認できない。
- Vacuumは全候補を毎回見るため、改善を増やしすぎると重くなる。`detour < 20` などの閾値依存が大きい。
- バックボーン評価にVacuum後の効果は入っていないため、Vacuumで強い構造を作る前提の遠回りをSAが見つけにくい。
