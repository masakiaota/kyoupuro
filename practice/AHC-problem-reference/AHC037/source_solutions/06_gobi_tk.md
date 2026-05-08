# AHC037 - gobi_tk 解法メモ

## 参照元

- 記事: AtCoder Heuristic Contest 037 ~Soda~ 参加記
- URL: https://gobi-tk.hatenablog.com/entry/2024/09/18/012709
- 著者: Gobi
- サイト: gobi_tkの日記
- 種別: 参加記、詳細解説、提出コード付き
- 成績・順位: 98位/985人。提出 #57820282 は Score 4,989,449,232、AC、実行時間約1877 ms
- コード有無: 記事内にAtCoder提出リンクあり
- コードを読めたか: 読めた。AtCoder 提出 https://atcoder.jp/contests/ahc037/submissions/57820282 を確認した
- 読めなかったもの: 記事中のX/Twitter引用は本文としては確認できた範囲に留めた。提出コードは読めた

## 解法の全体像

二段階の貪欲で構築する。まず、入力点だけを使って木を作る。選ぶ順序は `min(x,y)` が小さい点からで、壁際から詰めるように処理する。選んだ点は、既に選ばれている点のうちマンハッタン距離が最も小さい到達可能な点へ接続する。

次に、その木の辺を逆順に見ていく。ある辺 `a -> b` について、それより前に作った別の辺の最短経路上に中継点を置くと、元の辺のコストを保ったまま `b` への接続を短くできることがある。その場合、既存辺を中継点で分割し、対象辺を中継点から `b` へ張り替える。これを操作数上限か改善不能まで繰り返す。

## 主要アイデア

- まず入力点だけで到達可能な木を作る。
- 点の処理順は `min(x,y)` 昇順で、軸や壁に近い点から詰める。
- 辺を `Rectangle(leftup, rightdown)` として持つ。`leftup` が生成元、`rightdown` が生成先である。
- 既存辺の最短経路上に中継点を追加しても、その既存辺の総コストは悪化しない。
- 追加した中継点から別の点へつなぎ替えると、その別点へのコストだけ改善できる。
- 作成順の逆順に長い・後から作った辺を見直す。
- 強い貪欲に比べるとノード間マージの自由度が低く、エッジ間の繋ぎ替えに寄っている。

## 最終コードの構造

### 状態表現

- `Coord { x, y }`: 座標。`distance` と `can_make_to` を持つ。
- `Rectangle { leftup, rightdown }`:
  - 生成元から生成先への依存関係を表す。
  - `diagonal_dist()` は生成コスト。
  - `saitan_ten(b)` は、辺の最短経路上で点 `b` に近い中継点を返す。
- `State`:
  - `coords`: 作った点集合。初期は `(0,0)`。
  - `deps`: 現在の依存辺集合。
  - `ans`: 出力操作列。

### 観測・制約・入力の扱い

- 入力点は `goal.x.min(goal.y)` 昇順にソートする。
- 各目標点について、既に作った点 `c` のうち `c.x<=goal.x && c.y<=goal.y` を満たすものから、距離最小の点を親にする。
- 辺は `Rectangle::new(from, pos)` として `deps` に追加する。
- 操作数上限 `MAX_OPS_NUM = 5N` を超えないよう、繋ぎ替えループを止める。

### 評価関数

- 初期構築では、選択済み点から目標点へのマンハッタン距離を最小化する。
- 繋ぎ替えでは、対象辺 `repr` の現在距離 `repr.diagonal_dist()` より、既存辺上の中継点 `next` から対象先 `b` への距離が短ければ改善とする。

```text
current = dist(repr.leftup, repr.rightdown)
next = closest point to b on another edge's shortest path
improved_cost = dist(next, b)
if improved_cost < current:
    reconnect
```

### 探索・構築・更新

- 初期木:
  - `coords = [(0,0)]`。
  - 入力点を `min(x,y)` 昇順に見る。
  - 到達可能な既存点から距離最小の `from` を選ぶ。
  - `deps.push(from -> pos)`、`coords.push(pos)`。
- 辺繋ぎ替え:
  - `deps` のコピーを作る。
  - 後ろから辺 `repr` を見る。
  - `repr.rightdown = b` とする。
  - それより前の辺 `rect` について、`rect.leftup` が `b` を作れる左上側にあるか確認する。
  - `rect` の最短経路上で `b` に最も近い点 `next` を計算する。
  - `dist(next,b)` が現在距離より短い候補の中で最良を選ぶ。
  - 見つかれば、
    - `repr` を `next -> repr.rightdown` に置換する。
    - `rect` を `rect.leftup -> next` と `next -> rect.rightdown` に分割する。
    - `next` を作成点へ追加する。
    - 外側ループの最初からやり直す。

### 操作・クエリ・出力選択

- 最後に `deps` をそのまま `Command::Make(leftup, rightdown)` に変換する。
- `State::print()` は操作数として `coords.len()-1` を出し、`ans` を順に出力する。
- `deps` の順序は繋ぎ替えで既存辺を分割しているため、前から実行しても生成元が作成済みになるよう維持されている。

### 時間配分・パラメータ

- 焼きなましや乱択は使わない。
- 提出コードは 1.5 秒前後から 1.9 秒弱で動いている。
- 繋ぎ替えは改善が見つかるたびに最初から見直すため、改善回数と `deps` 長に依存して重くなる。
- 操作数が `5N` に達したら改善ループを止める。

## 実装上重要な断片

初期構築は次である。

```text
sort goals by min(x, y)
coords = [(0,0)]
for goal in goals:
    from = argmin distance(c, goal) over c in coords with c can make goal
    deps.push(from -> goal)
    coords.push(goal)
```

辺繋ぎ替えは次の形である。

```text
while len(deps) < 5N:
    changed = false
    for repr in reversed(deps):
        b = repr.to
        best = none
        best_dist = dist(repr.from, repr.to)
        for rect in deps before repr:
            if not rect.from can make b:
                continue
            next = closest point to b on rect path
            if dist(next, b) < best_dist:
                best = (rect, next)
        if best:
            replace repr by next -> b
            split rect into rect.from -> next and next -> rect.to
            coords.push(next)
            changed = true
            break
    if not changed:
        break
```

`saitan_ten` は、既存辺の矩形内で `b` に近い点を取る。

```text
next.x = min(rect.rightdown.x, b.x)
next.y = min(rect.rightdown.y, b.y)
```

## この解法の本質

この解法は、後ろ向きマージではなく、前向きに作った辺を「再利用可能な経路」として見る。マンハッタン距離では、既存辺の最短経路上に点を追加しても元の辺のコストは変わらない。したがって、その中継点を別の辺の親にできれば、ほぼ無料で共通部分を作れる。

Gobi自身も記事で反省している通り、これはノード同士を自由にマージする強い貪欲より制約が強い。エッジに中継点を差し込む方針なので、結合順序の自由度が初期木に依存する。それでも、初期木を `min(x,y)` で壁際から作ることで、後段の繋ぎ替えが効きやすい依存順序にしている。

## 真似するならまず実装する部分

まず入力点だけの初期木を作る。次に、各辺を `from,to` として配列に持ち、既存辺上の中継点で別辺を短縮できるかを1回だけ試す処理を作る。これが動いたら、改善が見つかるたびに辺を分割してループを再開する。

強い貪欲に進む前の中間方針として有用である。幾何的な中継点の効果を理解しやすく、出力も前向きの辺列のまま扱える。

## 注意点・未理解点

- `State::print()` は `coords.len()-1` を操作数として出し、`ans` は `deps` から作る。通常は一致する設計だが、修正時に `coords` と `deps` の数をずらすと壊れる。
- 辺の順序を壊すと、出力時に生成元が未作成になりWAになる。
- `saitan_ten` は既存辺の最短経路上の代表点を取る実装であり、経路の向きやL字の取り方を明示的に持つわけではない。
- 初期木に強く依存するため、最初の接続順序が悪いと後段の繋ぎ替えだけでは戻しにくい。
- 強い貪欲のような「2点をマージして新しいノードにする」発想には届いておらず、記事でも自由度不足が反省点として挙げられている。
