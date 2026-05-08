# AHC049 統合解法メモ

## このメモの位置づけ

このメモは、Browser取得キャッシュで実装を読めた3件を中心に、AHC049のグループ化解法を整理したものである。attgm と kicky は今回の提出一覧で該当ACまたはコードリンクを拾えなかったため、個別メモを削除し、参照から外した。

## 個別メモ一覧

- [01_moegi.md](source_solutions/01_moegi.md): 提出 #67103781。グループ分割SA、耐久判定の後ろ向き計算、alias sampling、non-sort 近傍、末尾区間swapが分かる。
- [02_mec2.md](source_solutions/02_mec2.md): 提出 #67043260。単独グループ初期解、近傍5x5からの移動/swap、重さ順評価、高速化とtabooの実装が分かる。
- [03_shymohn.md](source_solutions/03_shymohn.md): 提出 #75379714。move+insert、merge、2-opt、swap、splitを持つ可変グループSAが分かる。

## 実装キャッシュ反映

- Moegi: #67103781。`score=1/cost_sum`、温度0.99から0.0、事前リジェクト `delta>2`、non-sort移動25%・swap50%・末尾swap25%を確認した。
- mec2: #67043260。温度 `1e3 -> 1e-2`、1.98秒、近傍5x5、1個/2個の移動またはswap、最良保存が終盤80%以降だけであることを確認した。
- Shymohn: #75379714。1950ms、温度50から0.1、近傍比率 55/15/10/10/10、移動先挿入位置全探索、merge/split 両方を確認した。
- attgm/kicky: キャッシュなし。今回の指示に従い補完調査せず、source_solutions から削除した。

## 問題の本質

全箱を出口へ運ぶとき、1回の出口往復を1グループとみなすと扱いやすい。各グループは箱ID列で、列順が pickup 順になる。スコアはほぼ全グループの往復移動距離合計で決まり、耐久制約はグループ単位で検査できる。

操作2は3実装とも使わない。各グループを出口から開始して箱を拾い、出口へ戻る独立ルートとして出力する。

## 参照解法の比較

| 参照元 | 主方針 | 実装上の特徴 | 真似すべき要素 |
|---|---|---|---|
| Moegi | グループ順序込みSA | 箱ペアの重み付きサンプリング、局所距離悪化の事前リジェクト、末尾区間swap | non-sort近傍、`inv_groups`、後ろ向き耐久判定 |
| mec2 | 近距離グループSA | 近傍5x5から移動/swap、グループ内は重さ順で簡略評価 | 単純で再現しやすい初期実装 |
| Shymohn | 経路順序も探索するSA | move+insert、merge、2-opt、swap、splitで可逆性を確保 | 挿入位置全探索、merge/split の併用 |

## 実装するなら

最小実装は mec2 型でよい。

```text
start with one box per group
repeat by annealing:
    choose nearby groups
    move or swap 1-2 boxes
    recompute only touched groups
    reject durability violation
emit each group as one round trip
```

伸ばすなら、Moegi 型の順序を固定しない近傍と事前リジェクトを入れ、その後 Shymohn 型の挿入位置全探索、merge/split、2-opt を足す。

## 重要な実装要素

- `box_id -> (r,c,w,d)` と `dist[box_or_exit][box_or_exit]` を前計算する。
- `groups[g]` は pickup 順の箱列、`belong[box]` または `box_to_trip[box]` は所属グループ。
- グループ評価は移動距離と耐久違反を同時に返す。違反は大ペナルティではなく invalid として扱う。
- 差分更新は、触ったグループの旧コストを引き、変更後のグループだけ再評価し、棄却時はバックアップを戻す。
- 出力は各グループごとにマンハッタン移動、箱上で `1`、最後に出口へ戻る。グループ順は総距離にほぼ影響しない。

## 落とし穴

- 先に拾った箱が下、後に拾った箱が上である。耐久判定の向きを間違えやすい。
- 箱を拾う前の移動では、その箱自身の耐久は削れない。
- 空グループ削除や merge/split の後、所属配列更新を忘れると二重所属・未所属になる。
- グループ内順序を変えたら距離だけでなく耐久も必ず再評価する。
- 最後に出口へ戻らないと、持っている箱が搬出されない。

## 参照元

- https://atcoder.jp/contests/ahc049/editorial/13384?lang=ja
- https://mec2.hatenablog.com/entry/2025/06/22/170541
- https://note.com/shymohn/n/n16a747d474c2
