
これにそってやっていく
https://qiita.com/drken/items/e77685614f3c6bf86f44

ペース2日に1問

## メモ
### 全探索
#### ビット全探索
- ABC014_C
- ABC045_C

#### 深さ優先探索
- ATC001_A ... 超基本.マスの探索
- ARC031_B ... 類問。道具として深さ優先探索が使える
- ARC037_C ... グラフの問題。こんな使い方もできる。ちゃんとグラフを探索している問題。


ちょっとやって感じたことがある。再起がややこしいので、どういう探索をしているのかちゃんと頭の中で整理してから書き始めよう。
- 探索をしない条件はなにか
- なにかを見つけたとき何を上流に伝えたいのか
- すべてを探索し尽くしたときの挙動はどうするか
等をちゃんと考えると良い。

#### 幅優先探索
スタートからゴールまでの最短の手順を全探索するのは基本的にこれ。

- ABC007_C ... 基本の基本。マスの探索。基本はこの書き方である。

### 貪欲法
#### コインの問題
現実の硬貨ならば大丈夫だが、一般的には必ずしも貪欲が成り立つわけではない。
- JOI2007_A

#### 区間スケジューリング
仕事の開始時間と終了時間が与えられて、より多くの仕事をしたいという状況。

選べる仕事のうち、終了時間が一番はやいやつを貪欲に選んでいくのが最適となる。(帰納法と背理法で証明できるらしい)

- KUPC2015_A
- ABC103_D

#### 辞書順最小
- ABC076_C

#### その他の例題
交換して良くなることはあっても悪化しない系
- ABC083_C.py (丸め込み誤差がくそなやつ)
- ABC005_C.py
- ABC091_C.py 二次元量同士を比較する最大二部マッチング、Greedy に解けるマッチング例らしい
  - 実装がなにげにめんどくさい

fence repair は面白い問題だったけど例題が全然ないので省略する。

### 動的計画法
#### 01ナップサック問題

