# AHC042 - tomerun 解法メモ

## 参照元

- 記事: AHC042 乱択解法
- URL: https://topcoder-tomerun.hatenablog.jp/entry/2025/02/04/001531
- 著者: tomerun
- サイト: TopCoderの学習のお時間
- 種別: 上位解説、参加記、提出コード付き
- 成績・順位: 7位。記事中の整理後コードは本番6位相当
- コード有無: AtCoder提出リンクあり
- コードを読めたか: 読めた。https://atcoder.jp/contests/ahc042/submissions/62366929 を確認した
- 読めなかったもの: なし。記事中の「ソースコード」はHatenaキーワードリンクであり、実体は同じ段落のAtCoder提出リンクだった

## 解法の全体像

焼きなましやビームサーチを使わず、ランダム性のあるルールベースを時間いっぱい繰り返す。各試行では、外周に近い向き付き座標のリストをランダム順に見て、そこに鬼がいれば外へ追い出す処理を行う。

追い出す途中で福が邪魔なら、福を直交方向にずらす。さらに、隣や2つ隣のレーンにいる鬼を、余計な手数以上に得になる場合だけ相乗りさせる。注目鬼の後ろに鬼が続く場合は、まとめて追い出す。試行が詰んだら捨て、時間内に得られた最短操作列を出力する。

## 主要アイデア

- `(座標, 外周に近い方向)` を全列挙し、毎回シャッフルして処理順を変える。
- 注目位置に鬼があれば、その方向へ出す。
- 進路上に福がある場合、左右または上下のどちらかにランダムに逃がす。
- 福を逃がせない場合、その試行は失敗として最初からやり直す。
- 追い出し1手ごとに、隣・2つ隣の鬼を相乗りさせるか再判定する。
- 相乗りは、移動後に外周までの距離がどれだけ縮むかと、相乗りに使う手数を比較して決める。
- 注目鬼の後方に鬼が連なっているなら、さらに同じ方向へ連打してまとめて落とす。
- 1周して鬼数が減らなければ、その試行は諦める。

## 最終コードの構造

### 状態表現

- `@init_st`: 初期盤面。
- `@st`: 現在試行中の盤面。`EMPTY`, `ONI`, `FUKU` を整数で持つ。
- `@acts`: 現在試行中の操作列。
- `Action(dir, i)`: 方向と行・列番号。
- `Result(acts)`: 操作列と、その長さをスコアとして持つ。
- `@add_len`: 相乗り判定の基準を少し揺らす乱数値。
- `@cont`: 1マス空きの後ろにいる鬼まで続けて排出するかを決める乱数フラグ。

### 観測・制約・入力の扱い

- 初期盤面を読み、各試行の先頭で `@st` を初期状態へ戻す。
- `can_shift(dir,pos)` は、操作方向の外周に福がいないかだけを見る。
- `can_remove_fuku(sr,sc,dir)` は、連続した福をその方向へ逃がせるかを調べる。逃がし先の外側に別の福が詰まっている場合は不可とする。
- 操作は常に `shift` を通して盤面と操作列を同時に更新する。
- 詰み条件は「福を逃がせない」「操作列が現在ベスト以上になった」「1周で鬼数が減らない」など。

### 評価関数

この解法にはビームや焼きなましの評価関数はない。各試行の良し悪しは最終操作列長だけで比較する。

局所的な相乗り判定では、次のような考え方を使う。

```text
if len_to_out(after_carry_position) + extra_shift_cost
   < len_to_out(original_position):
    carry the oni
```

`@add_len = rand(3)` により、どこまで先の状態を見て得と判定するかが少し揺れる。

### 探索・構築・更新

- `solve(timelimit)`:
  - まず1回 `solve_one(4N^2)` を実行して初期ベストを作る。
  - 以後、前回の最悪実行時間を見ながら、時間内に収まる限り `solve_one(best_score)` を繰り返す。
  - より短い操作列が得られたらベストを更新する。
- `solve_one(best_score)`:
  - 各外周距離 `len` について、上下左右の処理対象 `(dir,pos,len)` を作る。
  - そのリストを毎周シャッフルする。
  - 縦方向なら `manip_vert`、横方向なら `manip_horz` を呼ぶ。
  - 全鬼が消えたら結果を返す。
  - 鬼数が減らなければ失敗を返す。
- `manip_vert` / `manip_horz`:
  - 注目位置に鬼がなければ何もしない。
  - 進行方向に連続している鬼を先頭側まで巻き込む。
  - 進路上の福を直交方向へどかす。
  - 1手ずつ押しながら、隣・2つ隣の鬼の相乗りをチェックする。
  - 後ろに続く鬼を、場合によって追加連打で落とす。

### 操作・クエリ・出力選択

- 操作は `shift(dir,pos)` で盤面を更新しながら `@acts` に追加する。
- 試行が成功したら `Result(@acts.dup)` を返す。
- 時間内の最短 `Result` を最後に出力する。

### 時間配分・パラメータ

- 時間いっぱい試行を繰り返す。記事では10万回程度回ったとある。
- `@add_len` は `0..2` の乱数。
- `@cont` は真偽を1/2で乱択。
- 相乗り元は隣と2つ隣まで。3つ以上離れた鬼は考慮しない。
- 既存ベスト以上の操作数になった時点でその試行を打ち切る。

## 実装上重要な断片

```text
solve_one(best):
    reset board and actions
    targets = all (direction, lane, distance_from_edge)
    while true:
        shuffle(targets)
        for target in targets:
            if vertical target:
                ok = manip_vert(target)
            else:
                ok = manip_horz(target)
            if not ok or actions.size >= best:
                fail this trial

        if no oni remains:
            return actions
        if oni_count did not decrease:
            fail this trial
```

福を逃がし、鬼を相乗りさせる部分は次の考え方である。

```text
before pushing target oni:
    for each fuku on the path:
        choose one perpendicular direction randomly
        if fuku can be shifted away:
            shift it away
        elif opposite direction works:
            shift it away
        else:
            fail

each push step:
    if adjacent lane has oni and shifting it into this lane is beneficial:
        shift adjacent lane once
    elif two-away lane has oni and two shifts are beneficial:
        shift adjacent lane twice
    shift target lane outward
```

## この解法の本質

この解法の本質は、問題を大域探索としてではなく、「今見つけた鬼を外へ出す小手順」の乱択反復に分解している点である。単純に近い外周へ押すだけなら弱いが、福の退避と鬼の相乗りを入れることで、1回の排出操作が複数の鬼を外へ進める操作になる。

また、評価関数を作らず、ランダムな処理順とランダムな細部選択を大量に試す割り切りも重要である。この問題ではN=20、鬼40体であり、1試行が軽い。細部に乱数を入れたルールベースを大量に回すだけでも、かなり良い局所構造に当たる。

## 真似するならまず実装する部分

まずは、福を落とさない `shift` と `can_shift`、そして「向き付き座標をシャッフルして、そこに鬼があれば近い外周へ押す」だけを実装する。次に、進路上の福を直交方向へ逃がす処理を入れる。

その後で、隣1マスの鬼を相乗りさせる処理を入れるのがよい。2つ隣、1マス空き後ろの連続排出、乱数パラメータは最後に足す要素である。

## 注意点・未理解点

- ルールベースなので、どの乱択が効いているかはパラメータ依存が大きい。
- `can_remove_fuku` は、福が連続しているケースや外周に福が詰まっているケースで壊れやすい。
- 相乗り判定の不等式は実装依存であり、`@add_len` の揺らぎも経験的な調整である。
- 1周して鬼数が減らない試行を捨てるため、遠回りすれば解ける局面は切り捨てられる。
- 盤面更新と鬼数更新を分けて持たない実装では、試行終了時の鬼数再計算が必要になる。
