# AHC008 - eijirou 解法メモ

## 参照元

- 記事: AHC008 参加記
- URL: https://eijirou-kyopro.hatenablog.com/entry/2022/02/27/003135
- 著者: eijirou
- サイト: eijirouの競プロ参加記
- 種別: 実装解説、提出コード
- 成績・順位: 記事記載の最終スコアは5,500,555,555。順位は本文からは確認できなかった
- コード有無: あり。記事中の提出リンク https://atcoder.jp/contests/ahc008/submissions/29649226 を確認した
- コードを読めたか: 読めた。`Simulator`、`Solver1`、`Solver2`、`Solver3`、固定フェンス生成、ロック領域、要求生成、終盤捕獲処理を確認した
- 読めなかったもの: 3種類のSolverの差分は反復的な固定配置が多く、全タスク座標の意図までは追っていない。提出コード自体は読めた

## 解法の全体像

細長い通路と小領域を固定的に作り、ペットがいる領域の出口を閉じる。最初にフェンス候補列を生成し、人に建設タスクを割り当てる。フェンス構築後、各ペットが属する領域から通路へ出るためのロック位置を求め、近い人を向かわせて入口を閉じる。

記事では、細長い通路でペットを捕まえる発想、スコア計算、低スコアケースの調査が説明されている。コードでは人数やペット数に応じて `Solver1`、`Solver2`、`Solver3` を切り替え、通路列の間隔や待機列を変えている。

## 主要アイデア

- 通路を作り、その両側の小領域をペットの閉じ込め先にする
- 壁そのものは `Task(x, y, d)`、つまり「人が立つマス」と「壁を置く方向」として管理する
- 初期フェンス構築は、人が空いたら次のフェンス列を割り当てる
- 通路以外の連結成分ごとに、その領域を閉じるための入口タスク `locks` を前計算する
- ペットがいる現在位置から `locks` を引き、必要な入口閉鎖を `demands` として作る
- 終盤は人を待機列に配置し、需要に近い人が入口を閉じる
- 人の数やペット数に応じて、通路の間隔や待機列を変える

## 最終コードの構造

### 状態表現

- `Task`
  - `x, y`: 人が立つ目的地
  - `d`: 壁を置く方向。`-1` は移動だけのタスク
- `Simulator`
  - `px, py, pt`: ペット位置と種類
  - `hx, hy`: 人の位置
  - `pg`, `hg`: 各マスのペット集合、人集合
  - `bg`: 壁
  - `po`: 犬猫の目的対象
  - `tasks[h]`: 人ごとのタスクキュー
  - `costs`: BFS結果
  - `pet_history`, `position_counter`: 実験・観測用
- `Fence`
  - `case1`, `case2`: どちら向きに作るかのタスク列
  - `adopt_case1`: 割当時に選んだ向き
- `Solver`
  - `fences`: 未構築の固定フェンス群
  - `is_block`: 最終的に壁になる予定のマス
  - `is_aisle`: 通路として残すマス
  - `locks[x][y]`: その領域を閉じるための入口タスク
  - `demands`: 現在ペットがいる領域から必要な閉鎖タスク

### 観測・制約・入力の扱い

- `Simulator` は問題文のペット移動を自前でシミュレートできる。ローカルテストでは乱数生成も持つ
- 実行時は毎ターン出力後にペット移動を読み、位置を更新する
- `can_put` は壁設置制約に加えて、全人が連結であり続けるかを `connected_all()` で確認する
- BFSは壁のみを障害物とする。人やペットの重なりは通行可能として扱う

### 評価関数

`Simulator::get_score` は、全人が連結である前提で、人0からの到達可能領域を使って近似的にスコアを計算する。

```text
reachable = BFS(human0)
cnt = reachable内のペット数
score = round(1e8 * reachable_size / (900 * 2^cnt))
```

複数人が同じ主領域にいる設計なので、問題文の平均を簡略化できる。最終的な方針は、全人を主通路側に残し、ペットだけを外すことにある。

タスク割当では距離が中心で、フェンス候補や閉鎖要求までのBFS距離、待機列から外れるペナルティ、犬猫なら早めに入口候補へ行かせる調整を使う。

### 探索・構築・更新

- `prepare`
  - 固定フェンス候補 `fences` を作る
  - `is_block` と `is_aisle` を初期化する
  - `is_block` と `is_aisle` を障害物として、残り領域ごとに `locks` を作る
- `make_fences`
  - タスクが空いた人に、次に作るべきフェンス列を割り当てる
  - フェンスは2方向の構築順 `case1/case2` を持ち、人から近い方を採る
- `h_act`
  - 現在タスク位置にいるなら壁を置く
  - 置けない場合、次タスクが同じ位置なら順序を入れ替えて試す
  - その後、目的地へBFSで移動する
- `update_demands`
  - 現在ペットがいる領域を見て、対応する `locks` を需要に変換する
  - 犬猫で入口が遠い場合は、壁を置くより待機タスクにすることがある
- `trap_pets`
  - 人を待機列に置きつつ、需要との距離を見て人へ閉鎖タスクを割り当てる
  - 終盤では待機列制約を弱めて、近い需要を優先する

### 操作・クエリ・出力選択

各ターンの流れは次の形である。

```text
if make_fences_phase:
    make_fences()
    if all fences assigned/built:
        trap_pets_phase = true

if trap_pets_phase:
    trap_pets()

ans = Simulator.h_act()
output ans
read pet moves and update
```

壁設置はタスクの目的地に到着してから行う。目的地へは `get_next` のBFSで1歩進む。人の移動では通路列を優先するようなタイブレークも入っている。

### 時間配分・パラメータ

- `last_phase_turn = 280` が設定され、終盤は待機列へのこだわりを弱める
- 通路列はSolverごとに異なる。`Solver1` は4本程度、`Solver2` は3本、`Solver3` は5本相当の待機列を持つ
- mainでは、人が8人以上なら `Solver3`、人が少なくペットが多い条件では `Solver2`、それ以外は `Solver1` を使う
- `manhattan_distance(task, pet) - 3` のようなマージンを使い、ペットが入口に近いほど急ぐ

## 実装上重要な断片

```text
prepare_locks:
    mark planned walls and aisles as blocked
    for each unvisited non-block cell:
        component = BFS(cell)
        lock_tasks = cells adjacent to aisle from this component
        for c in component:
            locks[c] = lock_tasks
```

```text
make_fences:
    for each idle human:
        idx = fence where this human is relatively closest
        choose case1 or case2 by distance
        push tasks of selected case
        remove fence
```

```text
update_demands:
    for each reachable pet:
        if locks[pet_cell] is empty:
            demand = move to aisle column
        else:
            demand = unbuilt lock tasks for pet component
        add margin based on distance from lock to pet
```

## この解法の本質

この解法の本質は、捕獲を連結成分の入口閉鎖問題に落としていることだ。固定通路を作れば、通路以外の領域はそれぞれ1つか2つの入口だけで主領域とつながる。そのため、ペット位置から「閉じるべき入口」を即座に引ける。

人は全員通路側に残るので、スコア計算も安定する。人が分断される複雑な構造を避け、主領域の広さを保ちながらペットだけを外す方針が分かりやすい。

## 真似するならまず実装する部分

まず `Task` と `locks` の考え方を実装するとよい。

1. 通路列と壁予定マスを決める
2. 壁予定マスを `Task` の列にする
3. 通路以外の連結成分ごとに入口タスク `locks` を作る
4. ペット位置から `locks` を引き、近い人に閉鎖を担当させる
5. 全人連結を保つ `can_put` を必ず入れる

この構造は、固定盤面型の中では比較的実装しやすく、デバッグもしやすい。

## 注意点・未理解点

- 3種類のSolverは似ているが、通路列と待機列の細部が違う。どの条件でどれが最善かは記事とコードだけでは完全には説明されていない
- `get_score` は全人連結前提なので、実装では本当に全人連結を保つ必要がある
- 通路を作る前にペットが壁予定地近くにいると、壁設置が待たされる
- `locks` が1つか2つである前提の固定構造を崩すと、需要生成が壊れる
- 犬猫は入口の近さだけでは処理しづらく、単純な通路構造では低スコアケースが残りやすい
