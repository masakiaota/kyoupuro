# AHC038 - titan23 解法メモ

## 参照元

- 記事: [AHC038 解法紹介（最終54位）](https://titan-23.hatenablog.com/entry/2024/10/17/044235)
- 著者: titan23
- サイト: titan-23
- 種別: 上位解説、ビームサーチ解法、提出コード
- 成績・順位: 最終54位
- コード有無: AtCoder最終提出リンクあり。C++23
- コードを読めたか: 読めた。Submission #58801409 を確認した
- 読めなかったもの: `solution_urls.md` 記載の `https://titan-23.hatenablog.com/entry/2024/10/15/113000` は404だった。同ブログの2024年10月アーカイブから同タイトルの正しいURLを特定して読んだ

## 解法の全体像

トンカチ型の木を固定し、差分更新型のビームサーチで操作列を求める。木は根0から関節1を伸ばし、関節1から全ての葉を生やす形で、根から関節までの距離と葉の長さを `N,V` に依存させる。各ターンの遷移は、根移動5通りと関節1の回転3通りを組み合わせ、葉は90度右・左も試して、掴める・置けるなら貪欲に操作する。

実装は通常の「状態を丸ごとコピーするビーム」ではなく、探索木上でActionを適用・ロールバックする差分更新型になっている。状態重複はZobrist風ハッシュで落とす。

## 主要アイデア

- 葉を多くして同時処理数を稼ぐため、関節1つから多数の葉を生やすトンカチ型にする。
- 葉長を全て異なるようにし、複数葉が同じ対象に干渉しにくくする。
- 評価は単純に「掴むと1、置くと1」。複雑な距離評価を使わず、進捗だけでビームを回す。
- 差分更新型ビームサーチで、状態コピーよりもAction適用・rollbackを中心にする。
- 状態ハッシュで同じ盤面・保持状態・根位置・姿勢を除外する。
- ビーム幅は `N,M` と残り時間から調整する。seed=0では幅17000程度を確保できたと記事にある。

## 最終コードの構造

### 状態表現

- グローバル入力: `N, M, V, S, T`。
- `par_init`, `dist_init`, `arms_init`, `arms_rots`: 決め打ち木と、回転後の各葉相対座標テーブル。
- `State`
  - `hash`, `score`
  - `root_y, root_x`
  - `is_hold`: 各葉が保持中か
  - `is_exist`: 残り初期たこ焼き
  - `is_empty`: 空き目的地
  - `dirs`: 各頂点の向き
- `Action`
  - 根移動 `dir`
  - 関節1の回転 `rot`
  - 各葉の右回転bit、左回転bit、take bit、put bit
  - rollback用の前後score/hash
- `TreeNode`, `SubState`: ビーム探索木のノードと次候補を表す。

### 観測・制約・入力の扱い

- 初期と目的が重なるマスは、`is_exist` と `is_empty` から除外する。
- 初期根位置は全マスを試し、初期Actionと次Actionの見込みが良い位置を選ぶ。
- 根移動候補 `op_actions` は、盤面外へ出ないものだけ事前列挙する。
- 葉の位置は `arms_rots[関節1方向][葉index][葉方向]` のような事前計算から取る。

### 評価関数

- `TAKE_SCORE` と `PUT_SCORE` を加算する進捗評価。
- `try_op` は新scoreを負値にして返し、`nth_element` では小さいscoreを良いものとして扱う。
- 完了目標 `IDEAL_SCORE` は、残りたこ焼き数に対して掴む・置く2回分を加えた値。
- ある程度ターンが進み、最良候補が `IDEAL_SCORE` に届いたら早期に解として採用する。

### 探索・構築・更新

- `decide_arms`:
  - 高密度なら葉間隔を少し詰める。
  - `D` を根から関節1までの距離、`D0` を葉長の刻みとして決める。
  - 葉は関節1から左右交互に異なる距離で伸ばす。
- `try_op`:
  - 関節1回転と根移動後に、各葉について現在位置、右回転後、左回転後を調べる。
  - 非保持なら `can_take`、保持中なら `can_put` を満たす最初の操作をAction bitに記録する。
  - 実状態は壊さず、新scoreと新hashだけを返す。
- `apply_op`:
  - Actionに記録された回転・移動・take/putを状態へ反映する。
- `rollback`:
  - Actionの逆操作で、盤面、保持、向き、根座標、score/hashを戻す。
- `BeamSearchWithTree`:
  - 探索木の各葉状態から候補Actionを作る。
  - `seen` で重複hashを除外する。
  - `nth_element` で上位 `beam_width` だけを残す。
  - 一本道になった探索木は実際に状態へ適用して根を進める。

### 操作・クエリ・出力選択

- 探索終了後、探索木の最良葉からrootまでのAction列を復元する。
- `record` でAction列を2V文字形式へ変換する。
- 初期状態で保持している葉がある場合に対応するため、最初に `P` だけの行を入れる実装になっている。

### 時間配分・パラメータ

- `MAX_TURN = 1000`。
- 初期 `BEAM_WIDTH = max(1, 10000 - N*N*10) * 2 + 2000`。`N=30, M<250` では増やす。
- 2500ms以降は、残りターン見積もりと1候補あたり時間からビーム幅を縮める。
- 2850ms以降はさらに幅を固定的に落とす。

## 実装上重要な断片

```text
try_op(action):
    save pre_score, pre_hash into action
    virtually rotate joint1 and move root
    for each leaf:
        if not holding:
            test current, R, L positions for remaining takoyaki
        else:
            test current, R, L positions for empty target
        record rotation bit and P bit in action
    restore virtual joint1 rotation
    return new_score, new_hash
```

```text
beam_on_tree:
    state follows committed root of search tree
    enumerate actions from current frontier
    drop duplicate hashes
    keep top beam_width substates
    append them as children of tree nodes
    prune invalid branches
```

## この解法の本質

状態コピーを避ける差分更新型ビームサーチが本質である。AHC038は1手の効果が大きく、ターン単位の文脈も強いため、焼きなましよりビームで複数の未来を残す方が自然という判断になっている。また、評価を掴む・置くの個数に絞り、木構造と葉貪欲で1手の質を担保している。

## 真似するならまず実装する部分

完全な差分更新型ビームから始めるのは重い。まず同じトンカチ型の木と、根移動5通り×関節1回転3通り×葉貪欲を、通常の状態コピー型ビームで実装するのがよい。次にハッシュ重複除外、最後にActionのapply/rollbackへ移行する。

## 注意点・未理解点

- `solution_urls.md` のURLは誤っていたため、正しい記事URLを別途特定して読んだ。
- 記事でも、関節数が少なすぎることが弱点だったと振り返っている。Vが大きいケースでは多関節化と枝刈りが必要そうである。
- `try_op` が状態を一時的に変えて戻すため、rollback漏れやhash更新漏れが致命的になる。
- 初期根位置選択、ビーム幅式、時間幅調整はコードから読めるが、細部のチューニング意図は一部推測である。
