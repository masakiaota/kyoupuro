# AHC030 - uta_ccc 解法メモ

## 参照元

- 記事: [AHC030参加記〖最終149位〗](https://utac.hateblo.jp/entry/2024/02/19/232118)
- 著者: uta_ccc
- 提出コード: [AtCoder submission 50414729](https://atcoder.jp/contests/ahc030/submissions/50414729)
- コード有無: あり
- コードを読めたか: 読めた

## 解法の全体像

この解法は、複雑なベイズ推定や相互情報量最適化よりも、1マス掘りによる正確な情報を使って油田配置候補を堅く絞る方針である。記事本文では、最終的には「掘って分かった0マスを含む配置を消す」「正マスを覆れる油田候補を調べる」「候補が確定した油田を配置する」という制約伝播が中心になっている。

提出コードも、各油田ごとに合法配置集合を持ち、各配置がどのマスを覆うかを管理している。掘った結果に応じて配置候補を削除し、候補集合から「必ず0」「油田である可能性が高い」などを判定して次に掘るマスを選ぶ。

この方針は上位の情報量型解法よりスコア上限は低いが、実装再現性が高い。`solution.md` からsub agentに実装させる用途では、強い最小実装として有力である。

## 主要アイデア

- 1マス掘りは高コストだが、得られる値は正確である。
- `v=0` のマスを含む配置は、その配置候補から除外できる。
- `v>0` のマスは、少なくとも1つの油田が覆っている必要がある。
- 各油田の合法配置候補を削っていくと、どのマスが油田になり得るかが分かる。
- 候補から一度も覆われないマスは `v=0` とみなせる。
- 確定した配置・確定した寄与が増えるほど、他の配置候補もさらに削れる。
- 複数マス占いはノイズがあるため、低ノイズ時の補助や、1マス掘りより安く広く見る用途に限定する。

## 最終コードの構造

### 状態表現

提出コードでは、形状、合法配置、掘削済みマス、候補状態を複数の配列で管理している。実装方針としては次の形に整理できる。

- `shape[m]`: 油田 `m` の相対座標リスト。
- `placements[m]`: 油田 `m` の合法配置一覧。
- `cover[m][p]`: 油田 `m` を配置 `p` に置いたときに覆うマス集合。
- `alive[m][p]`: その配置候補がまだ生きているか。
- `dug[cell]`: そのマスを掘ったか。
- `value[cell]`: 掘ったマスの実測値。
- `fixed_oil[cell]`: 既に油田ありと確定したマス。
- `fixed_zero[cell]`: 既に油田なしと確定したマス。

### 観測・制約の持ち方

`v=0` が出た場合、そのマスを覆る全配置を削除できる。

```text
on_dig_zero(cell):
    fixed_zero[cell] = true
    for m in oils:
        for p in placements[m]:
            if cover[m][p] contains cell:
                alive[m][p] = false
```

`v>0` が出た場合は、少なくとも `value[cell]` 個の油田がそのマスを覆っている。ただしどの油田かはまだ分からないので、即座に単純削除はしにくい。そこで、各油田ごとに「このマスを覆える生存配置があるか」を調べる。覆える油田が少なければ、その油田の配置を強く制限できる。

```text
on_dig_positive(cell, v):
    candidates = { m | exists alive p where cover[m][p] contains cell }
    if len(candidates) == v:
        for m in candidates:
            keep only alive placements of m that cover cell
```

これは記事中で説明されている「正マスから配置を確定していく」考えの最小形である。

### 評価関数

この方針では明示的な尤度関数より、制約伝播後の候補数やマスごとの候補被覆数を評価に使う。

- `possible_cover_count[cell]`: 生存配置のどれかで覆われる可能性のある数。
- `expected_cover[cell]`: 生存配置を一様とみなしたときの油田期待値。
- `uncertainty[cell]`: `cell` が油田かどうか未確定な度合い。

次に掘るマスは、期待値が高いマス、または不確実性が高いマスから選ぶ。記事の方針では、油田がありそうなマスを掘って正の制約を得ることが多い。

### 探索・更新

探索というより、掘削結果に応じた候補削除を繰り返す。毎ターン次を行う。

```text
loop:
    propagate constraints until no change
    if answer cells are determined:
        submit answer
    cell = choose_next_dig()
    v = dig(cell)
    add observation
```

`propagate` では、0マス、正マス、候補がなくなったマス、候補が一意に近い油田を使って、`alive[m][p]` を更新する。

### クエリ選択

基本は1マス掘りである。次に掘るマスは以下の優先度で選ぶ。

1. 生存候補から見て油田である確率が高い未掘削マス。
2. 正なら多くの候補を絞れ、0なら多くの配置を消せるマス。
3. 現在の回答候補に入るかどうかが不確実なマス。

複数マス占いは、低ノイズ時に未知マス集合の総量を安く見る補助として使える。ただしこの解法の主軸ではない。

### 回答判定

生存している配置候補のどれからも覆われないマスは `v=0` と推定できる。逆に、確定した油田配置や正マス制約から油田ありと分かったマスを回答に入れる。

実装上は、次のような近似回答が現実的である。

```text
answer = { cell | expected_cover[cell] >= threshold }
answer must include all dug cells with observed value > 0
```

回答に失敗した場合は、回答集合の境界付近、つまり `expected_cover` が閾値に近いマスを掘って補正する。

### 時間配分

制約伝播型なので、計算時間は軽い。スコアは掘削回数に支配される。したがって、重い探索を回すより、どのマスを掘れば最も候補が減るかを丁寧に評価する方が効く。

## 実装上重要な断片

配置候補からマスごとの期待被覆数を計算する処理が中心になる。

```text
compute_expected_cover():
    expected[cell] = 0
    for m in oils:
        alive_positions = alive placements of m
        for p in alive_positions:
            for cell in cover[m][p]:
                expected[cell] += 1 / len(alive_positions)
```

次に掘るマスは、0でも正でも情報が大きい場所を選ぶ。

```text
choose_next_dig():
    best_cell = none
    for cell not dug:
        p = probability cell has oil from expected_cover
        zero_gain = number of alive placements covering cell
        positive_gain = number of oils that can cover cell
        score = p * positive_gain + (1 - p) * zero_gain
        update best
    return best_cell
```

正マスの制約伝播は、完全に正確にやろうとすると組合せ制約になる。最小実装では「覆える油田数が観測値と等しいときだけ確定」など、安全な場合に限定するとよい。

## この解法の本質

本質は、ノイズ付き推定を避け、正確な局所観測を配置候補削除に変換することである。AHC030は油田形状が既知なので、1マスの `v=0` だけでも多数の配置を消せる。`v>0` も、覆える油田の候補が少なければ強い制約になる。

上位解法のような高精度ベイズ推定を実装できない場合、この制約伝播型は「全マス掘り」よりずっと良い最小解になり得る。とくにsub agentに実装させるなら、まずこの方針を明示した方が再現性が高い。

## 真似するならまず実装する部分

1. 各油田の合法配置を列挙する。
2. `alive[m][p]` を持つ。
3. `v=0` の掘削結果で、そのマスを覆る配置を削除する。
4. 生存配置から `expected_cover[cell]` を計算する。
5. `expected_cover` が高く、かつ配置削除効果が大きいマスを掘る。
6. 回答集合は `expected_cover >= threshold` とし、掘って正だったマスは必ず含める。
7. 回答失敗時は閾値付近の不確実マスを掘って再回答する。

## 注意点・未理解点

- 正マス `v>0` の完全な制約伝播は、油田の組合せを考える必要があり難しい。安全なケースだけ反映するのがよい。
- 1マス掘り中心なので、上位解法ほど低コストにはならない。
- ただし全マス掘りとは違い、候補を削るマスを選べば掘削数を大きく減らせる。
- 提出コード内の細かな優先度関数やパラメータは、記事本文だけからはすべての意図を断定できない。
