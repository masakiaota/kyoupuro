# AHC006 - scat_neko 解法メモ

## 参照元

- 記事: AHC006(AtCoder Heuristic First-Step 001) シンプルな焼きなましで本番23位相当をとる
- URL: https://scat-neko.hatenablog.com/entry/2025/03/28/190021
- 著者: scat_neko
- サイト: scat_nekoのブログ
- 種別: 実装解説、焼きなまし入門
- 成績・順位: Python 1,720,073 点、本番 69 位相当。C++ 1,897,635 点、本番 23 位相当。Rust 高速化版 1,950,966 点、本番 1 ページ目相当
- コード有無: 記事内コード断片あり、AtCoder 提出コードあり
- コードを読めたか: 読めた。Python https://atcoder.jp/contests/ahc006/submissions/64186075、C++ https://atcoder.jp/contests/ahc006/submissions/64184515、Rust https://atcoder.jp/contests/ahc006/submissions/64185940 を確認した
- 読めなかったもの: なし

## 解法の全体像

選択する 50 注文だけを焼きなましで最適化し、選ばれた注文の訪問順は毎回最近傍貪欲で作る。初期状態は注文 `0..49` を選択する。近傍では、選択中の 1 枠をランダムに選び、未選択のランダムな注文へ置き換える。その 50 注文に対して、現在位置から行ける最も近い点を選ぶ貪欲でルートと距離を再計算する。距離が短くなる場合、または焼きなまし確率で許される場合に置換を採用する。

ルート最適化を厳密にやらず、評価関数を軽い貪欲近似にすることで、注文集合の試行回数を増やす方針である。

## 主要アイデア

- 状態は選択済み注文 50 件の配列 `orders`。
- ルートは状態に含めず、評価時に `calc_distance_greedy(orders)` で作り直す。
- 各注文の訪問状態は `0=レストラン未訪問`, `1=配達先待ち`, `2=完了`。
- 最近傍貪欲では、状態 0 の注文はレストラン、状態 1 の注文は配達先を候補点にする。
- 近傍は「注文 1 件の入れ替え」だけである。
- 受理条件は `new_distance < current_distance` または `exp((current_distance - new_distance) / temp)`。
- 温度は `initial_temp=300`, `final_temp=1` の幾何的な減衰。
- C++ 化や Rust 化により、同じ方針でも反復回数が増えてスコアが伸びる。

## 最終コードの構造

### 状態表現

- `Point` が座標とマンハッタン距離を持つ。
- `restaurants` と `destinations` は注文番号で引ける座標配列。
- `orders` は長さ 50 の選択注文配列。
- Rust 版では `orders: [usize; 50]` と `in_orders: [bool; 1000]` を固定長配列で持ち、重複選択を O(1) で判定する。
- 評価中の訪問状態は `state[50]` に持つ。
- ルートは必要なときだけ `calc_route` で生成する。Rust 版では固定長配列 `[Point; 102]` に格納する。

### 観測・制約・入力の扱い

- 入力は 1000 注文をそのまま座標配列に保存する。
- 近傍で新しい注文がすでに選択済みなら、その試行は捨てる。
- 最近傍貪欲は、レストラン訪問前の配達先を候補に出さないため、pickup-before-delivery 制約を自然に守る。
- 最後にオフィスへ戻る点を必ずルートへ追加する。

### 評価関数

- 最小化するのは、最近傍貪欲で作ったルートの総距離である。
- これは選択注文集合の真の最短ルートではなく、軽量な近似評価である。
- 公式スコアは距離に単調なので、距離を短くすればよい。

### 探索・構築・更新

- 焼きなましループは制限時間まで回す。
- 1 回の遷移:
  - `exchange_idx` を 0..49 からランダムに選ぶ。
  - `new_order` を 0..999 からランダムに選ぶ。
  - すでに選択済みならスキップする。
  - `orders[exchange_idx]` を一時的に置き換える。
  - 最近傍貪欲で距離を評価する。
  - 受理なら現状態を更新し、棄却なら元の注文へ戻す。
- Rust 版は距離だけを計算する `calc_distance_only` と、出力用のルートを作る `calc_route` を分け、受理時だけルートを生成する。

### 操作・クエリ・出力選択

- 出力は最終的な `orders` と、対応する最近傍貪欲ルート。
- 注文番号は 1-indexed に変換する。
- 訪問点数は最大 `2*50+2=102` で固定的である。

### 時間配分・パラメータ

- Python/C++ 版の時間制限目安は 1.9 秒。
- Rust 版は 1.99 秒まで使う。
- 温度は `initial_temp=300.0`, `final_temp=1.0`。
- Rust 版は `ln` を使って温度計算を軽量化し、固定長配列と unsafe な走査で評価を高速化している。

## 実装上重要な断片

```text
calc_distance_greedy(orders):
    current = office
    state[i] = 0 for all selected orders
    repeat 2*M:
        candidates = restaurant(order[i]) if state[i] == 0
                     else destination(order[i]) if state[i] == 1
        move to nearest candidate
        state[i] += 1
    return distance + dist(current, office)

annealing:
    replace one selected order with random unselected order
    new_distance = calc_distance_greedy(orders)
    if better or random() < exp((old_distance - new_distance) / temp):
        accept
    else:
        rollback
```

## この解法の本質

この解法は、難しい順序最適化を「評価のための貪欲」に押し込み、焼きなましの探索空間を注文集合だけに絞っている。順序も同時に最適化する上位方針より粗いが、実装が短く、バグりにくく、評価を高速化しやすい。AHC006 は 1000 件から 50 件を選ぶ自由度が大きいため、多少ルート評価が近似でも、注文集合を大量に試せば良い候補へ寄っていける。

特に入門用としては、近傍、評価、受理判定の関係が明確で、焼きなましの効果を観察しやすい。

## 真似するならまず実装する部分

最初に `calc_distance_greedy(orders)` を正しく作るべきである。次に、注文 1 件入れ替えの山登りを作り、最後に温度付き受理へ変える。高速化は、重複判定の `in_orders`、距離だけ計算する関数、固定長配列の順に入れるとよい。

## 注意点・未理解点

- 最近傍貪欲は真の最適ルートではないため、評価上よく見える注文集合が実際の最良とは限らない。
- 記事脚注でも触れられている通り、簡略化した説明では「現時点の best」の保持を省略している。提出コードの変数名も `best` と `current` の役割がやや混ざっており、再実装時は「現状態」と「全期間ベスト」を明確に分けるべきである。
- C++ 版は選択済み判定に `find` を使うため、Rust 版の `in_orders` より遅い。
- 受理時だけルート生成する最適化を入れる場合、最後に出す注文列とルートが同じ状態に対応しているかを確認する必要がある。
