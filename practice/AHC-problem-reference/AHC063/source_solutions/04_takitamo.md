# AHC063 - takitamo 解法メモ

## 参照元

- 記事: [AHC063参加記](https://takitamo-swelog.blogspot.com/2026/04/ahc063.html)
- 著者: takitamo
- サイト: takitamo_software.log
- 種別: 詳細解説、提出コード
- 成績・順位: 最終153位、perf 1900
- コード有無: あり。記事内の最終提出 [AtCoder submission 74920908](https://atcoder.jp/contests/ahc063/submissions/74920908)
- コードを読めたか: 読めた。Kotlin、AC、実行時間1730ms、コードサイズ53939Byte
- 読めなかったもの: なし

## 解法の全体像

ルールベースを土台に、正しい餌を連続して取れる部分だけをビームサーチで伸ばす解法である。最終コードでは、滑り止めの蛇行解、ルールベースのダイクストラ解、ビームサーチ解を用意し、時間内に複数乱択を回して、完成していて短いものを採用する。

基本ループは、まずビームで正しい接頭長を伸ばせるだけ伸ばす。伸びなくなったら、色ずれがあれば噛みちぎって復元し、正しい餌が難しければコスト付きダイクストラでとにかく次色へ向かう。噛みちぎりが発生したら、切れた尾を辿って復元するが、完全復元の少し手前で止めて再度ビームに任せる。

## 主要アイデア

- ビームサーチ対象を「間違った餌を含まず、正しい餌だけを順番に取れる状況」に絞る。
- それ以外の難しい状態はルールベースで処理する。
- 間違った餌を食べたら、頭から間違い位置までのどこかを噛みちぎって、正しい位置まで復元する。
- 正しい餌が見つからない場合、非目標餌や胴体を高コストにしたダイクストラで次色へ向かう。
- 噛みちぎり後は尾側の位置列を保存し、それを辿って復元に使う。
- 完全復元より少し手前で止めると、その後のビームが別ルートへ分岐しやすくなる。
- BFSの上下左右順をランダム化し、同じ最短距離でも違う入り方を試す。
- N/Mが小さく時間が余るケースでは、乱数シードを変えて多数試行する。

## 最終コードの構造

### 状態表現

- `State`
  - `ouroboros`: 蛇本体。
  - `field`: 盤面。
  - `lastRouteNode`: 移動列復元用の親ポインタ。
  - `_matchIndex`: 目標列と一致している最後のindex。
- `Ouroboros`
  - `dataArray` に座標row、座標col、色を分けて入れるリングバッファ。
  - `getShape(i)`, `getColor(i)` で頭からのi番目を参照。
  - `move(dir, field, turn)` が移動・食事・噛みちぎりを処理し、切れた尾の座標列を返す。
- `Field`
  - `food[index]`: 餌色。
  - `step[index]`: そのマスに最後に頭が来たターン。
  - `step` は噛みちぎり判定とBFS中の体残存判定に使う。
- `Result`
  - `RouteNode` から移動列を復元して出力する。

### 観測・制約・入力の扱い

- 初期状態では、盤面の餌配列を `Field` に入れ、蛇を固定初期位置で作る。
- `State.move` は、餌を食べて体長が伸びたときだけ `_matchIndex` を更新する。噛みちぎりで短くなった場合は `_matchIndex` を体長に合わせて縮める。
- 噛みちぎり判定は `turn - field.step[next]` と現在体長から、まだ体が残っているかを推定する。

### 評価関数

ビームサーチ内の主評価は、コードでは次の順序で状態を選んでいる。

- `matchIndex` が大きいほど良い。
- 同じ `matchIndex` なら `turn` が少ないほど良い。

ルールベースのダイクストラでは、辺コストを次のように変える。

- Uターンは不可。
- 非目標餌は高コスト。
- まだ体が残っているマスはさらに高コスト、場合によっては通行不可。
- それ以外はコスト1。

記事上の「最も多くの餌を取り続けられるルート、同数ならターン数が少ないルート」と一致している。

### 探索・構築・更新

- `LeastStrategy`
  - 最悪時の滑り止め。単純な蛇行で移動する。
- `GreedyDijkstraStrategySafe`
  - 高コストダイクストラで次に欲しい色へ向かう。
  - 色ずれ中はランダムに間違い位置より頭側の体を噛む候補へ向かう。
  - 噛みちぎり後、切れた尾を可能な範囲で復元する。
- `BeamSearchStrategy`
  - `tryToCreateLongerStreak` で、非目標餌・噛みちぎりを避けながら次目標色へのBFS候補を作る。
  - ビーム幅は経過時間で200から4程度まで段階的に減らす。
  - 次状態は `matchIndex` 降順、`turn` 昇順で残す。
  - `searchForCut` は噛みちぎれる位置をBFSで全列挙し、候補からランダムに選ぶ。
  - `execute` は、ビームで伸ばす、色ずれならcut、必要ならダイクストラで次色へ向かう、という優先順で進む。

### 操作・クエリ・出力選択

- `main` ではまず `LeastStrategy` を作る。
- 次に `GreedyDijkstraStrategySafe` を実行して初期bestにする。
- その後、乱数seed `0..1000` と `tailOffsetMode` 2種類で `BeamSearchStrategy` を回し、完成していて移動列が短い解を採用する。
- 例外や時間切れが起きたら、それまでのbest、なければ滑り止め解を出す。

### 時間配分・パラメータ

- `defaultLimit = 1600` ms。
- ターン上限は100000。
- ビーム幅は経過時間比で、おおむね200、160、100、75、50、35、20、10、4と小さくなる。
- 噛みちぎり復元は、切れた尾を完全には戻さず、最終コードでは尾側を8個程度残して止める分岐が使われている。
- `tailOffsetMode` は2種類試す。
- 乱数seedを多数試すが、時間判定で途中停止する。

## 実装上重要な断片

```text
main:
    fallback = LeastStrategy()
    best = GreedyDijkstraSafe()
    for seed in 0..1000 while time remains:
        for tail_offset_mode in [1, 0]:
            cand = BeamSearch(seed, tail_offset_mode)
            if cand.completed and cand.turns < best.turns:
                best = cand
    print(best or fallback)
```

```text
beam_extend(state):
    targets = bfs_paths_to_next_color_without_wrong_food(state)
    for target_path in targets:
        next = copy(state)
        apply path
        if next.matchIndex increased:
            candidates.add(next)
    keep by (-matchIndex, turn)
```

```text
on_bite(detached_tail):
    while detached_tail.size > 8:
        next_pos = detached_tail.pop_front()
        if next_pos adjacent to head and its food color is desired[current_len]:
            move_to(next_pos)
        else:
            break
    return to main loop / beam search
```

## この解法の本質

この解法の本質は、ビームサーチを適用する範囲を「正しい餌だけを素直に取れる局面」に限定し、それ以外をルールベースで強引に整える分業にある。評価関数設計を複雑にせず、ビームでは一致接頭長と手数だけを見ればよい状態へ問題を切り出している。

また、噛みちぎり後に完全復元しない判断が重要である。復元は安全だが、完全に戻すと同じ詰まり方に戻ることがある。途中で止め、そこからビームに任せることで、新しい経路が生まれる。

## 真似するならまず実装する部分

1. `food` と `step` を持つ正確なシミュレータ。
2. 非目標餌と体を高コストにしたダイクストラ。
3. 噛みちぎり後、切れた尾を辿って復元する処理。
4. `matchIndex` 最大、同点なら手数最小の単純ビーム。
5. BFS方向順と噛みちぎり候補の乱択。

Kotlin提出コードは構造が読みやすいので、上位解法より先に「ルールベース+ビーム」の実装練習として向く。

## 注意点・未理解点

- `step` を使った体残存判定は高速だが、噛みちぎり直後や頭尾重複を雑に扱うと壊れやすい。
- `bypassMode` の迂回復元はコード上に残っているが、最終ループでは実質無効化されている。記事の「迂回はあまり効かなかった」と対応する。
- 完全復元を何手手前で止めるかはパラメータ依存が大きい。
- `if result2.isCompleted && result2.route.size < best.route.size` の選択なので、未完成だがスコアがよい候補は最終選択から落ちる可能性がある。
