# AHC033 - bowwowforeach 解法メモ

## 参照元

- 記事: [トヨタ自動車プログラミングコンテスト2024#5（AtCoder Heuristic Contest 033）参加記](https://bowwowforeach.hatenablog.com/entry/2024/05/30/210002)
- 著者: bowwowforeach
- サイト: bowwowforeachの日記
- 種別: 1位ユーザ解説、上位解説、公開提出コード
- 成績・順位: 1位。scorerun では延長戦スコア 49,754,594,067、提出時刻 2024-05-27 16:02:06 とされている
- コード有無: 記事本文には最終提出リンクなし。記事内にビットボードBFSのサンプルコード画像あり。BrowserでAtCoder提出一覧 `https://atcoder.jp/contests/ahc033/submissions?f.User=bowwowforeach` を開き、System Test提出 [#53968407](https://atcoder.jp/contests/ahc033/submissions/53968407) と同時刻の通常提出 [#53942556](https://atcoder.jp/contests/ahc033/submissions/53942556) を辿って読んだ
- コードを読めたか: 読めた。C++23、コード長 89387 Byte、`TaskTreeState` と時間軸 `Frame`、`Finder`、ビームサーチ実装が中心だった
- 読めなかったもの: 記事本文から直接は最終提出に辿れなかった。パラメータ調整の根拠はコード上の値と記事の説明からの推定を含む

## 解法の全体像

一時置き回数を抑える搬入順序をDPで下準備し、その順序から大きく外れない範囲で「1台のクレーンが1個の箱を拾い、搬出口または一時置き場へ運ぶ」タスクを列挙する。各タスクについて、時間軸つき盤面上で到達可能性を計算し、実行後状態を評価してビームサーチする。

最終コードでは、状態はクレーン位置、箱の配置、各搬入口の進捗、各搬出口の進捗、時間ごとのクレーン・箱占有を持つ。候補は「クレーン ai が箱 box を取り、位置 to へ置く」アクションであり、経路探索で実行可能性を確認したうえで状態に登録される。重複状態はZobrist Hashでまとめ、同じ状態なら評価の悪い候補を捨てる。

## 主要アイデア

- 搬入順序は `(5+1)^5` 程度の状態DPで扱い、一時置きの少なさと搬出可能性を見積もる。
- クレーン経路は `x,y,t` の3次元空間で探す。5x5固定なので、6列化したビットボードを使って移動可能集合をビットシフトで更新する。
- ビームサーチの1手は、ターン単位の1操作ではなく「箱を拾って置く」まとまったタスクにする。
- タスクの割り当て順が違っても箱配置・進捗が同じなら同一状態とみなし、Zobrist Hashで枝刈りする。
- 一時置き回数最小の順序に固執せず、待ち時間を減らすために多少の一時置きを許す。
- 一時置き場所、搬入・搬出進捗、クレーン完了時刻、マンハッタン距離からのロスなどを線形・非線形に重み付けして評価する。

## 最終コードの構造

### 状態表現

- `TaskTreeState` / `TaskTree`
  - 搬入口ごとの搬入数 `ins_` を状態番号化し、どの行から次に搬入するかと一時置きの悪さを前計算する。
- `Frame`
  - 各ターンの占有状態を持つ。`noArmGrid_`、`noBoxGrid_`、`armCrossMask_` により、クレーン衝突とすれ違い禁止を表す。
- `CommanderState`
  - `frames_`: 時刻ごとの盤面制約。
  - `ins_`: 実際の搬入進捗。
  - `boxTurn_`: 各箱が拾えるようになる時刻。
  - `puttableTurn_`: 各マスに箱を置けるようになる時刻。
  - `EvalState`: クレーン空き時刻・位置、箱グリッド、箱位置、一時置きビット、搬出進捗、評価差分、Zobrist Hash。
- `CommanderAction3`
  - `ai_`, `box_`, `to_`, `seed_`。1つのビーム遷移を「どのクレーンがどの箱をどこへ置くか」として表す。

### 観測・制約・入力の扱い

- 入力 `A` は `server.ques_` に保持する。
- 搬入口の次箱は、箱が入口から動いた時刻に初めて出現するものとして `boxTurn_` と `frames_` を更新する。
- 搬出口に置く候補は、その行で次に搬出すべき番号だけに限定する。
- 小クレーンが箱を持っているときは、経路探索時に `noBoxGrid_` も通行制約に入れる。大クレーンは箱をまたげる。
- すれ違い禁止は、時刻ごとの `armCrossMask_` を更新して、逆方向移動を禁止する形で表現する。

### 評価関数

評価は高いほどよい形式で、各ペナルティを減算し、進捗・位置評価を加算する。

- 減点:
  - 総ターン数 `totalTurnScore`
  - 拾いに行く経路のマンハッタン距離からのロス
  - 運ぶ経路のマンハッタン距離からのロス
  - 一時置き回数
  - 搬入順DPから見た悪さ `badLevel`
  - 拾い・運搬に使った総ターン
- 加点または差分評価:
  - クレーン空き時刻評価
  - 搬入進捗評価
  - 搬出進捗評価
  - 一時置き位置評価
- 追加制約:
  - 中央3列に全行が埋まるような一時置きは避け、少なくとも1行は通路として空ける。

### 探索・構築・更新

- 汎用 `BeamSearch<CommanderState, CommanderAction3>` を使う。
- `EnumCandidate` で候補箱を集める。
  - DPが推奨する搬入口の先頭箱。
  - 盤面上の一時置き箱。
  - 搬出可能箱は搬出口へ。
  - まだ搬出できない搬入口上の箱は、搬出口に近づく一時置き候補へ。
- `Finder` が箱への到達時刻、置き先への到達時刻、経路復元を担当する。
- `PreCalcRouteWithPreCalc` で拾い・置きの距離を見積もり、`EvalNextState` で差分評価とハッシュを計算する。
- `Regist` が実行確定した経路を `frames_`、箱位置、搬入出進捗、アクション列に反映する。
- ビーム幅は序盤固定、以降は残り時間と現在処理量から動的に決める。

### 操作・クエリ・出力選択

- ビームが保持する最良アクション列を `CommanderState::RegistToResult` で各クレーンの `P/Q/UDLR` に復元する。
- タスク列の最後で、短いクレーン列には `B` を追加して他クレーンの邪魔をしないようにする。
- 出力は各クレーンごとの操作文字列である。

### 時間配分・パラメータ

- 制限時間は `TIME_LIMIT=2900ms`。
- `InitialBeamWidth=7000`、状態容量は `bs.Init(800000, 40000)`。
- `FrameCap` はコード上で最大探索ターンを固定長配列として扱う。
- `SkipOverTurn`、`ArmCutTurn`、`AppendTurn` などで候補の遅れや探索幅を制限する。
- 評価重みは `HP` に多数まとめられており、記事の通り手動調整とOptuna調整が入っていると見られる。

## 実装上重要な断片

```text
beam_step(state):
    boxes = recommended_input_heads + stacked_boxes
    for crane in usable_cranes:
        get_turns = bitboard_bfs_to_boxes(state.frames, crane)
        for box in boxes:
            if box is next output:
                try to = output_cell(box)
            else if box is at input:
                try to in temporary_cells_closer_to_output
            route = find_pick_and_carry_route(crane, box, to)
            if route feasible:
                next_eval, next_hash = evaluate_delta(state, route)
                push_to_beam(next_eval, next_hash, action)
```

```text
apply(action):
    reserve crane path in frames
    when box leaves input, expose next input box
    if destination is output:
        advance output progress
    else:
        mark temporary cell occupied from release time
    update crane free turn, crane pos, box pos, hash, eval components
```

## この解法の本質

ターン列を直接探索せず、問題を「実行可能なタスク列」に持ち上げている点が本質である。搬入順DPでタスク列の大枠を作り、3次元BFSで物理的な実行可能性を保証し、ビームで局所的な割当順を比較する。箱とクレーンの衝突制約は重いが、N=5固定を利用したビットボードと時間別フレームに落とせば、かなり強い候補生成を時間内に回せる。

また、重複状態を「どの順でそこに来たか」ではなく「今の箱配置・進捗・クレーン位置」でまとめるため、ビーム幅を有効に使える。同じ配置なら、途中で余計な待ちや迂回をした経路は評価で自然に負ける。

## 真似するならまず実装する部分

まずは3次元BFSで「あるクレーンが箱を拾って指定マスに置けるか」を判定し、経路を出力列へ反映できるようにする。次に、搬入口進捗5次元DPで一時置き最小の搬入順を出し、搬出可能箱は搬出口、そうでなければ中央の一時置きへ置く候補生成を作る。

その後で、ビームサーチ、Zobrist重複除去、評価値の重み付けを足すのがよい。最初から全パラメータを再現しようとすると、衝突予約と時間管理で破綻しやすい。

## 注意点・未理解点

- 記事本文には最終提出リンクがなく、提出 #53968407 / #53942556 はBrowserでAtCoder提出一覧から辿ったものだ。
- 評価重みの根拠は記事でも「試して調整」と説明されており、コードを読んでも各値の必然性までは分からない。
- `Frame` の時刻更新、搬入口の次箱出現、`Q` と次の `P` の詰め方はWAの温床である。
- 一時置き候補を増やすと探索は強くなるが、小クレーンの通路を壊して経路探索が急に通らなくなる。
- ビットボードは6列化・マスク向き・時刻境界を間違えると、すれ違い禁止や盤外移動を壊しやすい。
