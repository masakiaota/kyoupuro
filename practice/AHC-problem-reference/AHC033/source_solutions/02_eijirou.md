# AHC033 - eijirou 解法メモ

## 参照元

- 記事: [AHC033 参加記](https://eijirou-kyopro.hatenablog.com/entry/2024/05/30/110801)
- 著者: eijirou
- サイト: eijirouの競プロ参加記
- 種別: 3位ユーザ解説、上位解説、公開提出コード
- 成績・順位: システムテスト 1,971,936,410,473 点、3位
- コード有無: 記事内に提出コードリンクあり。AtCoder公開提出 [#53968444](https://atcoder.jp/contests/ahc033/submissions/53968444)
- コードを読めたか: 読めた。Rust、約46KB、ビーム幅可変のビームサーチ実装だった
- 読めなかったもの: なし。ただし、評価係数の細かな調整意図は記事とコードから完全には復元できない

## 解法の全体像

最終解法はビームサーチである。1ステップは「1台のクレーンがコンテナの場所へ移動し、掴み、運び、離す」まとまったタスクとして扱う。盤面上にある各コンテナを候補にし、最も早く拾えるクレーンと、搬出口またはスタック位置までの経路をBFSで求める。遷移後の状態をスコア下界ベースの評価関数で評価し、Zobrist Hashで重複を落としながら幅2万前後のビームを進める。

スタックは1コンテナ1回まで、スタック位置は2列目・3列目に限定する。搬出可能なコンテナは必ず搬出候補にし、搬出できない搬入口上のコンテナだけをスタック候補にする。

## 主要アイデア

- ビーム1手をターン1操作ではなく、箱1個の運搬タスクにする。
- 経路探索は `x,y,t` の3次元BFSをビットボードで高速化する。
- 拾いに行くBFSは状態ごとに全クレーン同時に1回行い、どのクレーンが最短で拾えるかを出す。
- 置きに行くBFSは候補ごとに行い、採用が見えてから経路復元する。
- 評価値は「全コンテナ搬出までに必要な労働力の下界」を基礎にし、スタック位置など下界で見えない悪さを補正する。
- Zobrist Hashで、搬入済み集合、スタック箱と位置、大クレーン位置、小クレーン位置集合をまとめ、同一状態の悪い候補を捨てる。
- `QP` のような「置いて即拾う」無駄操作を削る過去改変を入れる。

## 最終コードの構造

### 状態表現

- `Environment`
  - `receive_order`: 入力順。
  - `receive_pos`: コンテナ番号から搬入口・深さへの逆引き。
  - `receive_cost`: 搬入進捗5次元状態から残り労働力下界を返すDP。
  - `stack_hashes`, `crane_hashes`, `receive_hashes`: Zobrist Hash用乱数。
- `State`
  - `max_actions_len`, `sum_actions_len`: ターン最大長と全クレーン行動量。
  - `actions`: 各クレーンの操作列。
  - `crane_pos`: 各クレーン位置。
  - `grid`: 各セルにあるコンテナ番号、空なら番兵。
  - `receive_progress`, `dispatch_progress`: 搬入・搬出進捗。
  - `used_by_crane`: 時刻ごとのクレーン占有ビット。
  - `used_by_container`: 時刻ごとのコンテナ占有ビット。
  - `masks`: すれ違い禁止を反映した方向別移動マスク。
  - `stack_pos`, `stack_turn`, `last_turn`: スタック位置と時刻制約。
  - `zobrist_hash`: 重複除去用ハッシュ。
- `Candidate`
  - 評価値、元状態ID、クレーンID、経路の位置を持つ。
- `Selector`
  - ビーム上位候補をセグメント木で保持し、同一ハッシュの悪い候補を除外する。

### 観測・制約・入力の扱い

- `Environment::init_receiving_cost` が搬入進捗状態ごとの残りコストを後ろ向きDPで前計算する。
- 搬入口から箱を取ると `receive_progress` が進み、次の箱が入口に出る。
- 搬出口に置くと `dispatch_progress` が進む。置けるのはその行で次に必要な番号だけである。
- 到着条件として、搬入・搬出・スタック生成時刻・小クレーンが箱を持って通った時刻より後であることを要求する。
- 小クレーンが箱を持つ場合は、BFSでコンテナ占有セルを通過不可にする。

### 評価関数

コード上の基本形は次のような下界評価である。

```text
cost = max(
    5 * max_actions_len,
    sum_actions_len
      + big_crane_extra
      + receive_cost[receive_progress]
      + stack_cost
      + stack_position_cost
      - crane_pos_bonus
)
```

補正要素:

- `receive_cost`: 残り搬入・搬出・最低限のスタックに必要な労働力。
- `stack_cost`: スタックした箱を将来搬出口へ運ぶ距離と `PQ` 分のコスト。
- `get_stack_pos_cost`: スタックがある行数、スタック数、同じ行に並ぶ同一搬出先コンテナの相性を評価する。
- `BIG_CRANE_COST`: 大クレーンの労働力を少し重めに扱う。
- `crane_pos_bonus`: 右側へ進んだクレーン位置を少し評価する。

記事の説明では、通れなくなった行数のペナルティが特に大きい。スタックが全行に散ると小クレーンが通りにくくなるためである。

### 探索・構築・更新

- `main` は現在ビーム `curr_states` から全候補を出し、`Selector` で上位を選んで `act` する。
- ビーム幅は時間で `27000, 25000, 22000, 18000, 10000` と変わる。
- `push_candidates` が全セルのコンテナを見て候補を作る。
  - その箱が搬出可能なら搬出口へ運ぶ候補。
  - 搬入口上でまだ搬出できないなら、2列目・3列目の空きスタック候補へ置く候補。
  - スタック済みでまだ搬出できない箱は動かさない。
- `bfs_with_all_cranes` が、各クレーンの現在時刻から同時に到達可能ビットを広げる。
- `get_pick_up_crane` が、目的セルに最初に到達できるクレーンと時刻を復元する。
- `release_then_push` が置き先までのBFS、評価、ハッシュ、経路復元、候補登録を行う。
- `act` が採用候補の経路を実状態へ反映する。

### 操作・クエリ・出力選択

- 各候補の経路は `directions` に保持し、採用後に該当クレーンの操作列へ追加する。
- 搬入セルで `P` した場合は、次の搬入箱を `grid` に反映する。
- 搬出口で `Q` した場合は、該当行の `dispatch_progress` を進める。
- 最終的に完了状態のうち `max_actions_len` が最小の操作列を出力する。

### 時間配分・パラメータ

- `TIME_LIMIT_SEC = 2.9`。
- `INTERNAL_MAX_TURN = 100`、`BFS_ABORT_TURN = 20`。
- スタック列は `STACK_Y = [2, 3]`。
- ビーム幅は序盤大きく、終盤に小さくする。
- スタック行数コスト、Linear Conflictコスト、スタック数コスト、大クレーンコストなどは定数で調整されている。

## 実装上重要な断片

```text
push_candidates(state):
    pickup_reachable = bfs_from_all_cranes_over_time()
    for cell containing container:
        if stacked and not dispatchable:
            continue
        crane, pickup_turn = earliest_crane(cell)
        if dispatchable:
            try_release(output_cell)
        if cell is input:
            for stack_cell in columns 2,3:
                if can_stack(stack_cell):
                    try_release(stack_cell)
```

```text
try_release(to):
    end_turn = bfs_with_time_reservation(pickup_turn + 1, from, to)
    cost = evaluate_next_state(delta)
    hash = stack_receive_hash_delta ^ crane_hash_delta
    if selector accepts:
        reconstruct pickup path and release path
        push candidate
```

## この解法の本質

本質は、スコアそのものではなく「全員であと何労働力必要か」という下界で状態を比較することだ。クレーン同士の干渉は下界には完全には入らないが、最大操作長とのmax、スタック位置ペナルティ、Zobristによる多様性で補う。5台のクレーンをターンごとに直接同時最適化するのではなく、1台のまとまった運搬を遷移単位にしたことで、3日実装でも上位に届く形になっている。

## 真似するならまず実装する部分

最初は次の3つでよい。

- `State` に箱位置、搬入・搬出進捗、クレーン操作列、時刻別占有を持たせる。
- 1箱を拾って置く経路を3次元BFSで作る。
- 評価値を `max(5*max_turn, sum_actions + 残り搬出の下界)` にする。

スタック位置評価やZobrist、`QP` 削除、動的ビーム幅はその後で足すべきだ。

## 注意点・未理解点

- `used_by_crane` と `used_by_container` は時刻が1ずれるとWAになる。搬入・搬出がターンの前後どちらで起きるかを常に確認する必要がある。
- スタックを2列に限定する単純化は強いが、置き先が詰まったときの枝切れに注意する。
- 経路復元は採用直前まで遅らせると速いが、復元用の到達ビットを保持し忘れるとデバッグが難しい。
- Zobrist Hashは経路を含めないため完全一致ではない。記事では、その方が多様性が出て良かったとされている。
- 評価係数はかなりチューニング依存で、再実装時は大量seedで確認する必要がある。
