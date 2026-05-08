# AHC010 - takumi152 解法メモ

## 参照元

- 記事: ALGO ARTISプログラミングコンテスト(AHC010) 参加記
- URL: https://takumi152.hatenablog.jp/entry/2022/04/26/124706
- 著者: takumi152
- サイト: takumi152の競プロ日記
- 種別: 上位解説、参加記、提出コード付き
- 成績・順位: 1位/1400人、最終スコア 20,218,968 点
- コード有無: 記事内に最終提出リンクあり
- コードを読めたか: 読めた。AtCoder 提出 https://atcoder.jp/contests/ahc010/submissions/31233938 を確認した
- 読めなかったもの: 記事中の X/Twitter 埋め込みは本文取得できないものがあった。ただし記事本文と提出コードで主要な解法は確認できた

## 解法の全体像

初期盤面にすでに存在するループから、盤面中央に近い2本を選ぶ。それぞれのループを、通るタイルとそのタイルから出る方向の列として保持する。

以後は焼きなましで2本のループを伸ばす。近傍は「片方のループからランダムな連続区間を取り除き、その両端をつなぐ別経路を乱択 DFS で探す」だけである。DFS が成功したら、その区間を新経路に差し替え、2本の長さから評価値を計算して採択する。前半は2本の長さの線形和、後半は本来のスコアである長さの積を使う。

この解法は、最初から全盤面を構築し直すのではなく、成立している2閉路を保ったまま局所的に壊して作り直す。出力は最終盤面の各タイル状態を、元のタイルから何回回転させたかに変換して作る。

## 主要アイデア

- ループを `(x, y, 出方向)` の列として明示的に持つ。
- 盤面 `board` と各タイルが2本のループで何回使われているかを `tile_use_count` で持つ。
- 初期解は、初期状態で存在するループを列挙し、中心に近いものを2つ採用する。
- 近傍は「ループ区間の部分破壊 + DFS 再構築」である。
- DFS はタイル種別と使用回数で遷移可能方向を分ける。
- 曲線1本タイルは未使用なら左右どちらにも曲げられるが、使用済みなら通れない。
- 曲線2本タイルは未使用なら回転を仮置きしながら通し、1回使用済みなら残っているレール側だけ通す。
- 直線タイルは未使用なら直進だけ許し、使用済みなら通らない。
- 評価関数は前半と後半で切り替える。前半は短い方を強く伸ばす線形評価、後半はスコアそのものを使う。

## 最終コードの構造

### 状態表現

- `board_orig`: 入力の元タイル状態。
- `board`: 現在解として採用しているタイル状態。
- `ans`: `board_orig` から `board` への回転回数。
- `tile_use_count[30][30]`: 2本のループが各タイルを何回使っているか。曲線2本タイルは2回使われ得る。
- `path_info[2]`: 2本のループ。各要素は `(x, y, dir)` で、そのタイルから出る方向を表す。
- `rail_next_dir[tile][dir]`: ある方向から入ったとき、次に出る方向。
- `rail_rotation_id_table[orig_tile][from_dir][to_dir]`: 元タイルを、入方向から出方向へつながる状態にするためのタイル ID。
- `rail_rotation_count_table[orig_id][now_id]`: 出力用の回転回数変換。

### 観測・制約・入力の扱い

- 入力は30行の数字を読み、`board_orig` と `board` に持つ。
- `is_loop(start_x, start_y, start_dir)` で、現在盤面上にループが存在するか確認する。
- 初期解では全タイルを走査し、`start_dir = 0` から閉じるループを見つける。見つけたループは中心距離で優先度付けする。
- 記事によれば、採点用テストでは初期盤面に少なくとも2つループがある前提で実装している。一般には初期ループが1本以下のケースもあり得る。
- DFS 中は一度取り除いた区間の `tile_use_count` を減らし、候補経路探索中に仮使用して戻す。

### 評価関数

- 前半評価:

```text
eval1(len0, len1) = 10 * min(len0, len1) + max(len0, len1)
```

- 後半評価:

```text
eval2(len0, len1) = len0 * len1
```

- 前半でスコアそのものを使わない理由は、積の評価では高スコアになるほど1手の悪化幅が大きくなり、温度が同じでも悪化遷移を受け入れにくくなるためである。
- 2本の長さのバランスも重要なので、前半では短い方を強く評価している。

### 探索・構築・更新

- 初期構築:
  - 盤面上の既存ループを列挙する。
  - 中央から近いループを2本選ぶ。
  - それぞれを `path_info` に展開し、通過タイルの `tile_use_count` を増やす。
- 近傍:
  - 2本のうち1本をランダムに選ぶ。
  - そのループ上の2点 `id1, id2` をランダムに選び、連続区間を取り除く。
  - 区間の始点側から終点側へ、乱択 DFS で新しい経路を探す。
  - DFS のステップ上限は 300。
- 採択:
  - 新評価が現評価以上なら必ず採択する。
  - 悪化なら `exp((new_score - old_score) / temperature)` で採択する。
  - 不採択なら、減らした `tile_use_count` を戻す。
- 差し替え:
  - 新経路の各方向に対して、現在タイルを `rail_rotation_id_table` で入出方向に合う状態へ更新する。
  - `path_info` の区間を削除し、新経路列を挿入する。

### 操作・クエリ・出力選択

- 操作は各タイルの回転回数を900文字で出力するだけである。
- コードでは `calc_ans_from_board()` が、元タイルと現在タイルから回転回数を引く。
- `output_ans()` は30x30の `ans` を1行に連結する。
- 焼きなまし中にベスト更新時の出力も行っているが、解法上の本質は「現在盤面を出力文字列へ変換できる状態で常に保持する」点である。

### 時間配分・パラメータ

- 前半焼きなまし:
  - 評価は `eval1`
  - 温度は 200 から 2 へ指数的に下げる
  - 時間上限は 0.680 秒
- 後半焼きなまし:
  - 評価は `eval2`
  - 温度は 1000 から 100 へ指数的に下げる
  - 時間上限は 1.980 秒
- DFS ステップ上限は 300。
- 近傍種類は1種類のみで、選ぶループ、区間、DFS の左右探索順を乱択する。

## 実装上重要な断片

```text
initialize:
    board = input_board
    loops = find_existing_loops(board)
    choose two loops nearest to center
    path_info[0], path_info[1] = their edge sequences
    tile_use_count = usage by the two loops

anneal(score_function, time_limit):
    while time remains:
        lo = random loop id
        id1, id2 = random interval on path_info[lo]
        decrement usage of removed interval

        next_path = randomized_dfs(start_endpoint, target_endpoint, step_limit)
        if not found:
            restore removed interval usage
            continue

        new_score = score_function(new_len0, new_len1)
        if accept(new_score, current_score, temperature):
            rotate tiles along next_path
            replace interval in path_info[lo]
            current_score = new_score
        else:
            restore removed interval usage
```

DFS のタイル別処理は次のように考える。

```text
dfs(cell, in_dir):
    if outside or step_limit exceeded:
        fail
    if straight tile:
        can go straight only when unused
    if single-curve tile:
        can choose either left/right turn only when unused
    if double-curve tile:
        if unused:
            choose one curve and temporarily rotate tile
        if used once:
            follow the still unused rail determined by current rotation
        if used twice:
            fail
```

## この解法の本質

この解法の本質は、閉路という壊れやすい構造を、常に2本の明示的な閉路として持ち続ける点である。盤面全体を毎回スコア計算してから良し悪しを決めるのではなく、ループ列の長さだけで評価できる形にしている。

また、タイルを1枚ずつ独立に回すのではなく、ループの一部を「経路」として壊して、両端を結ぶ別経路を探す。これにより、途中状態が閉路でないことを許しつつ、採択時には必ず閉じたループへ戻せる。長い閉路を作る問題では、単点変更よりも、区間破壊とパス再接続の方が探索空間に合っている。

評価関数を前半と後半で変える点も重要である。積を最初から最大化すると、長さのバランスや悪化許容が扱いにくい。まず2本を伸ばし、最後に本来スコアへ寄せる構成になっている。

## 真似するならまず実装する部分

まずは次を実装するのがよい。

- `to[t][d]` によるループ追跡と長さ計算。
- 既存ループを列挙し、中心に近い2本を `path_info` として保持する部分。
- `tile_use_count` を使った「区間を一旦未使用にする」処理。
- ステップ上限付き DFS で、区間の両端をつなぐ別経路を1本見つける処理。

焼きなましは最初から完全に作らなくてもよい。まずは「見つかった新経路が長くなるなら採用する」山登りで、閉路を壊さず更新できるか確認する。その後、温度付き採択と前半・後半評価の切り替えを足す。

## 注意点・未理解点

- 初期盤面に2本以上の既存ループがない場合のフォールバックは、記事・コードでは本質的には扱っていない。
- `path_info` の `dir` が「入方向」なのか「出方向」なのかを混同すると、DFS の始点・終点指定が壊れる。
- 曲線2本タイルは同じタイルを2回使えるため、タイル座標列だけでは状態が足りない。タイル内のどちらのレールを使ったか、または入出方向まで含める必要がある。
- 区間を外した後に DFS が失敗した場合、`tile_use_count` を必ず戻す必要がある。
- DFS 中に曲線2本タイルの回転を仮更新するため、失敗時の復元を間違えると盤面と `path_info` がずれる。
- ベスト解の保持と出力まわりは提出コード固有の挙動がある。再実装するなら、標準的に `best_board` を別に持ち、最後に1回だけ出力する方が分かりやすい。
