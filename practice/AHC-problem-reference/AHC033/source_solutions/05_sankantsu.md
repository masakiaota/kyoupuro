# AHC033 - sankantsu 解法メモ

## 参照元

- 記事: [AHC033 参加記](https://sankantsu.hatenablog.com/entry/2024/05/29/022511)
- 著者: sankantsu
- サイト: sankantsuのブログ
- 種別: 71位参加記、詳細解説、GitHub公開コード
- 成績・順位: AtCoder解説一覧では71位
- コード有無: 記事内にGitHubリンクあり。[sankantsu/competitive-programming ahc033/src/main.rs](https://github.com/sankantsu/competitive-programming/blob/master/ahc/ahc033/src/main.rs)
- コードを読めたか: 読めた。Rust、約1000行。GitHub rawも確認した
- 読めなかったもの: なし。ただし記事にある「特定ケースで残るバグ」の具体例はコードだけでは再現確認していない

## 解法の全体像

ターンベースの貪欲である。現在状態から次に搬出したいコンテナ、またはそれを出すために搬入口から出す必要があるコンテナを目標にする。クレーンへ目標マスを割り当て、各クレーンの次の1手候補をBFSで作り、全クレーンの組合せを列挙して衝突しない中で進捗が最大のものを選ぶ。

一時置き場の使い方と搬入順の見方を複数パターン試す。中央行を空けるため、行探索順の24通り、搬入順方針3通り、列優先か行優先か2通りを組み合わせた144パターンを時間内に試し、最短ターンの解を採用する。

## 主要アイデア

- 盤面シミュレータを厳密に作り、`step` で問題の操作制約を検証しながら進める。
- 次に搬出したい箱がキュー奥にある場合、その手前の箱を一時置き対象として出す。
- 一時置き場所は主に2列目・3列目を使い、中央行を避けることで通路を残す。
- 行探索順、列優先/行優先、搬入順方針を全探索して、詰まりにくいパターンを選ぶ。
- 各ターンの移動は、全クレーンの可能な次手を直積で列挙し、衝突しない候補から距離改善が最大のものを選ぶ。
- 終盤は残り箱すべてをクレーンに割り当て、搬出順が逆転しないように待たせる。

## 最終コードの構造

### 状態表現

- `Input`
  - `n` と搬入順 `a`。
- `Move`
  - `Stay`, `Pick`, `Release`, `Move(dir)`, `Bomb`。
- `Crane`
  - 大クレーンか、位置、保持中コンテナ。
- `ContainerState`
  - `Done`, `Carrying(i)`, `Queue(i, depth)`, `Board(x,y)`。
- `State`
  - `queue`: 各搬入口に残っている箱。実装ではpopしやすいように逆順で持つ。
  - `done`: 各搬出口から搬出済みの箱列。
  - `board`: 盤面の箱。
  - `cranes`: クレーン状態。
- `Solution`
  - クレーンごとの操作列。
- `Solver`
  - 入力と現在 `State` を持ち、各パターンを試す。

### 観測・制約・入力の扱い

- `State::carry_in` は、入口セルが空で、そこに箱を持ったクレーンがいなければ次箱を出す。
- `State::carry_out` は右端セルの箱を即座に `done` へ移す。
- `State::step` は操作を同時に適用し、以下を検証する。
  - 空セルでないと `Release` 不可。
  - 箱がないと `Pick` 不可。
  - 小クレーンが箱を持つ場合、箱のあるセルへ移動不可。
  - 複数クレーンの同一マス到着とすれ違い不可。
  - 爆破済みクレーンの操作不可。

### 評価関数

明示的な状態評価関数より、各ターンの局所選択で進捗を最大化する。

- `bfs(from,to,move_over)` は、最初の1手ごとに目的地までの距離を返す。
- `consider_next_move` は全クレーンの候補を直積で列挙し、距離和が小さい候補を選ぶ。
- `validate_turn_action` は衝突・すれ違いに加え、右端付近で搬出順が逆転する動きも避ける。
- パターン全体の評価は `Solution::len()`、つまりターン数。

### 探索・構築・更新

- `determine_recv_order(chunk_size)`
  - chunk単位で各搬入口から何個出すかを全列挙し、連続して搬出可能になる箱数を増やす行順を貪欲に作る。
- `determine_target_containers_on_demand`
  - 事前搬入順を使わない場合、次に搬出したい各行の箱について、キューから掘り出す数が少ないものを優先する。
- `make_destinations`
  - 目標箱の状態に応じて、盤上ならその位置、キュー奥なら搬入口を目的地として列挙する。
- `search_free_cells`
  - 指定された行探索順で、2列目・3列目の空き一時置きセルを列挙する。空になった搬入口も一時置き候補に追加する。
- `match_crane_with_target`
  - 箱を持っているクレーンは、搬出可能なら搬出口へ、そうでなければ空き一時置きへ向かわせる。
  - 空きクレーンは、目標マスに近いものから割り当てる。
- `consider_next_move`
  - 各クレーンの次手候補を作り、直積列挙で最良の同時操作を選ぶ。
- `solve`
  - 行探索順24通り、搬入順3通り、列優先/行優先2通りを試す。2.8秒で打ち切る。

### 操作・クエリ・出力選択

- 現在地が目的地で、箱を持っていなければ `Pick`。
- 現在地が目的地で、箱を持っていれば `Release`。
- 目的地がないクレーンは `Stay` と移動候補を持つ。右端に居座らないように軽いコストを付ける。
- 終盤は `assign_all_remaining_containers` で残り箱を各クレーンへ割り当て、`consider_next_moves_in_last_phase` で順序待ちを入れる。
- 解の操作列はターンごと配列からクレーンごとの文字列へ転置して出力する。

### 時間配分・パラメータ

- 2.8秒を超えたらパターン探索を打ち切る。
- 初期 `max_turn = 120`。良い解が出たら最大許容ターンを短くし、以降はそれより悪い探索を切る。
- 試すパターン数は最大 `4! * 3 * 2 = 144`。
- 搬入順は空リスト、chunk 4、chunk 5 の3種。
- クレーン数は常に5台を使う。

## 実装上重要な断片

```text
solve:
    for row_order in permutations(rows_except_center):
        for recv_order in [on_demand, chunk4, chunk5]:
            for col_major in [false, true]:
                reset state
                simulate until all carried out or timeout
                keep shortest solution
```

```text
consider_next_move:
    for each crane:
        if no target: candidates = stay + movable neighbors
        else if at target: candidate = P or Q
        else: candidates = bfs first steps toward target
    for combination in product(candidates):
        if valid no-collision and no-swap:
            score by remaining distances
    return best combination
```

## この解法の本質

この解法の本質は、ビームサーチを使わずに「毎ターンの同時操作の整合性」を小さな全探索で解くことだ。盤面が5x5でクレーンも5台なので、各クレーンの次手候補をかなり絞れば、直積列挙でも現実的に回る。長期的な詰まりは、一時置き利用順の144パターンを試すことで吸収する。

また、シミュレータを明示的に作っているため、操作制約に対して見通しがよい。これは実装量こそ増えるが、AHC033のようにWA条件が多い問題では重要である。

## 真似するならまず実装する部分

最初に `State::step` 相当のシミュレータを作り、任意の同時操作を検証できるようにする。次に、次に搬出したい箱とキュー奥の邪魔箱を目標化し、`bfs` で1手候補を出す。最後に、全クレーン候補の直積から衝突しない最良手を選ぶ。

パターン144通りや終盤専用処理は、基本貪欲が安定してから足すとよい。

## 注意点・未理解点

- 記事では、行き先が衝突したクレーン同士が止まる・振動するケースが残っており、複数パターン探索で回避していると説明している。
- `validate_turn_action` は右端での逆転も見ているが、これだけで全ての順序問題を防ぐわけではない。
- 一時置き場所を増やすと搬入は楽になる一方、小クレーン経路が壊れやすい。
- `bfs` は現在盤面の静的障害物を使うため、未来の他クレーン移動までは完全には見ていない。
- 終盤の `busy` 判定は順序逆転を避ける工夫だが、厳密な最適性はない。
