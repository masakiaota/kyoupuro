# AHC025 - ymatsux 解法メモ

## 参照元

- 記事: [AHC025: 統計的推測によらない4位解法の解説](https://note.com/ymatsux/n/n09938d7ab387)
- 著者: ymatsux
- サイト: note
- 種別: 上位解説、提出コード
- 成績・順位: 4位
- コード有無: あり。記事内の提出コードリンク [Submission #46854482](https://atcoder.jp/contests/ahc025/submissions/46854482)
- コードを読めたか: 読めた。C++20、約1280行で、主要な操作・キャッシュ・枝刈り・LDM部分を確認した
- 読めなかったもの: note記事内の画像の細部は完全には読んでいない。コードはコンパイル実行まではしていない

## 解法の全体像

重さを確率的に推定しきるのではなく、天秤比較で改善が証明できる局所操作を繰り返す。袋を軽い順に保ち、一番軽い袋と一番重い袋を中心に、1個移動、1個交換、Largest Differencing Methodによる2袋再構築を試す。

`Q` が大きい場合は最初に全アイテムをマージソートし、重さ順を得る。重さ順が分かると、失敗済み操作の単調性を使った枝刈り、近い重さの交換相手の探索、初期解生成ができる。`Q` が足りない場合でも、単一アイテム比較から得た大小関係をbitsetで推移閉包し、部分的な順序情報として使う。

## 主要アイデア

- 袋を軽い順に並べ、一番軽い袋と一番重い袋の差を縮める
- 操作A: 重い袋から軽い袋へ1個移す
- 操作B: 軽い袋の軽いアイテムと、重い袋の重いアイテムを交換する
- 操作C: 操作A/Bが行き詰まったら、2袋の和集合にLDMを適用する
- 同一クエリと左右反転クエリをキャッシュする
- アイテム単体の大小関係は `vector<bitset<N>>` で推移閉包する
- `Q` が大きい場合は全アイテム順序を先に取り、失敗済み操作から単調にスキップする
- `Q` が小さい場合は、交換候補をソートしながら近いペアだけ試す

## 最終コードの構造

### 状態表現

主な状態は `Main` クラスにまとまっている。

- `assignment_[item]`: アイテムが属する袋ID。袋IDは軽い順に再マップされる
- `item_rank_[item]`: 全アイテムをソートできた場合の重さ順位
- `item_order_matrix_[i][j]`: `w(i) < w(j)` が分かっているかを表すbitset行列
- `query_cache_`: 集合比較のキャッシュ。左右反転も保存する
- `invalid_move_table_[item]`: このアイテム以上の移動が以後失敗すると分かる情報
- `invalid_swap_table_[i][j]`: 交換失敗から導いた枝刈り表
- `pairwise_shuffle_failure_count_`: 順序未確定時のランダム再分配の失敗回数
- `multi_ldm_done_`: 初期LDMを実行したかどうか

### 観測・制約・入力の扱い

- `QueryByItemVectorsWithDuplication` で左右の共通アイテムを取り除き、空集合なら比較を推論する
- 同じ左右集合、または反転した左右集合は `query_cache_` から返す
- 単一アイテム比較なら `item_rank_` や `item_order_matrix_` からクエリなしで返せる場合がある
- 新しい単一比較が得られたら、bitset版Floyd-Warshallで推移関係を更新する
- `q_index_ >= q_count_` の場合は `"*"` を返し、これ以上クエリできないことを上位処理に伝える

### 評価関数

数値重さを推定してスコアを計算するのではなく、操作後の比較で改善を判定する。改善は主に「新しい重い側が、以前の重い側より軽いか」で確認する。

初期化時だけ、アイテム順位から単純な期待重さを作ってLDM風の構築に使う。記事では「統計的推測によらない」としているが、この初期解部分には順位にもとづく素朴な重さ見積もりが入っている。

### 探索・構築・更新

- `InitializationWithItemRankCost()` で、アイテムソート・袋ソート・余裕クエリを足した必要量を見積もる
- 足りる場合は `ItemMergeSort` で全アイテム順位を得る
- さらに余裕があれば、初期解にmulti-way Largest Differencing Methodを使う
- それが無理なら、順位から作った期待重さでLDM風にD分割し、その後袋をソートする
- 足りない場合は `i mod D` 初期解にして袋だけソートする
- メインループでは `RunSingleUpdatePhase()` を繰り返す
- 袋ペアは、軽い端と重い端に近いほど優先されるようなスコアで並べる
- 操作A、操作B、最後にLDMの順に試す

### 操作・クエリ・出力選択

操作Aでは、最軽袋が `group_0` のときだけ、重い袋から候補アイテムを移す。候補移動後の2袋を比較し、改善するなら採用する。重い端・軽い端の組で移動に失敗したアイテムは、順位情報から以後の重いアイテムも失敗するとみなして枝刈りする。

操作Bでは、順位が分かる場合は軽い袋・重い袋のアイテムを順位順に見て、失敗済み交換より差が大きいものを飛ばす。順位がない場合は、2袋のアイテムを比較しながらソート済み列へ挿入し、隣接する「重い袋側の軽いアイテム」と「軽い袋側の重いアイテム」だけを交換候補にする。

LDMは、対象2袋の和集合を比較ベースのタプルとして管理し、2つずつ取り出して差を作る形で再分割する。最後に新しい重い側が以前より軽ければ採用する。

### 時間配分・パラメータ

- 初期アイテムソートをするかは、必要クエリ数と `N` 程度の余裕で判断する
- `group_count_` が大きいほどLDMのクエリコストが重い
- 操作A/Bが十分失敗した後にLDMを試す
- 使い切れなかったクエリは `1 1 0 1` で消化する

## 実装上重要な断片

```text
query(L, R):
    remove common items
    if one side empty: infer
    if singleton order is known: infer
    if cache has L,R or R,L: reuse
    ask balance
    cache both directions
    if singleton comparison:
        update bitset transitive closure
```

```text
run_update_phase():
    group_pairs = pairs ordered from extremes
    for (light, heavy) in group_pairs:
        if can_move_from_heavy_to_light():
            apply and reinsert changed bags
            return true
        if can_swap_between_light_heavy():
            apply and reinsert changed bags
            return true
        if enough queries:
            candidate = LDM(light + heavy)
            if candidate improves:
                apply and reinsert
                return true
    return false
```

```text
after_failed_move(item):
    if item_rank is known:
        mark item and heavier items as invalid for future move
    else:
        use known item_order_matrix to mark dominated items
```

## この解法の本質

天秤比較で証明できる単調性を徹底して使っている点が本質である。特に、移動や交換に失敗した事実は「もっと重い移動」「もっと差が大きい交換」も失敗するという強い枝刈り情報になる。重さを数値化しなくても、順序と失敗履歴だけで候補集合をかなり減らせる。

また、LDMを「数値がないから使えない」と諦めず、比較関数で実装している点も重要である。典型的な数分割近似を、天秤比較だけで動く構築・局所改善に翻訳している。

## 真似するならまず実装する部分

最小実装としては、次の順がよい。

1. `assignment` と袋ソート
2. クエリキャッシュと共通アイテム除去
3. 最重・最軽の1個移動
4. 単一アイテム比較の推移閉包
5. 1個交換と失敗枝刈り
6. 余裕があればLDM

全アイテムソートや初期LDMは、基本の山登りが動いてから足す方が壊れにくい。

## 注意点・未理解点

- `invalid_move_table` と `invalid_swap_table` は単調性の向きを間違えると、有効な候補を全部捨てる危険がある
- `assignment_` の袋IDを軽い順に再マップし続けるため、更新後の参照先を間違えやすい
- LDMは比較回数を多く使うため、残りクエリ見積もりを誤ると最終出力前に詰まる
- note記事の図の細部は読めていないが、提出コードから実装の主要部は確認できた
