競プロをやる
===

### コピペ用
```python
#入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline

def read_ints():
    return list(map(int, read().split()))

def read_a_int():
    return int(read())

def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]

def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read() for _ in range(H)]

def read_col(H,n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp=list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    
    return ret


# A列、B列が与えられるようなとき
AB = [readln() for _ in range(M)]
A = [ab[0] for ab in AB]
B = [ab[1] for ab in AB]
#こういう風にスマートに内包表記を書くよりも
A = []
B = []
for _ in range(M):
    a, b = read_ints()
    A.append(a)
    B.append(b)
#愚直にappendしていったほうがかすかに早い

#再帰の上限を緩和する(引数は適当)
import sys
sys.setrecursionlimit(1 << 25)

#デフォルト値を持つ辞書
from collections import defaultdict
dic = defaultdict(lambda: 0)

#要素の数え上げ(文字数の数え上げに便利
from collections import Counter
dic = Counter(なんかの配列)

#1次元累積和
from itertools import accumulate
A=list(range(10)) #なんからのリスト
A_accum=list(accumelate(A)) #Aの累積和のリストがこれでできる
```

その他コピペで便利そうなアルゴリズムは`algorithm/`に保存してある。

### 手法実装例へのリンク
- [深さ優先探索](https://github.com/masakiaota/kyoupuro/blob/master/AtcoderBeginnersSelection/novice/ARC031_B.py)
  - [深さ優先探索を用いた閉路判定](https://github.com/masakiaota/kyoupuro/blob/master/AtcoderBeginnersSelection/novice/ARC037_B.py)
- [幅優先探索]( https://github.com/masakiaota/kyoupuro/blob/master/algorithm/BFS.py )
- [UnionFind木](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/UnionFind.py)
  - [具体的に問題でつかってのはこれ](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC120/D_correct.py)
- [ビット全探索](https://github.com/masakiaota/kyoupuro/blob/master/AtcoderBeginnersSelection/novice/ABC045_C.py)
- [二分探索]( https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC119/D_correct.py )

- [priority queue (コピペ用)](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/PriorityQueue.py)
  - [実際に使われた問題](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC123/D_correct5.py)

### 難
#### BeginnersSelectioin/novice
- ABC045_C問題 ... ビット全探索。考え方は難しくないが、ビット全探索という操作の書き方はなれておくと便利そう。また文字列をpythonスクリプトとして実行することもしたので必要になったときは見るべし。


#### ABC070
- C問題 ... 最小公倍数を求めるのに最小公約数をいかに効率よく求められるかが鍵
- D問題 ... 木構造の探索の問題。実装では深さ優先探索を採用した。

#### ABC062
- C問題 ... 長方形にたいして長方形を使ってなるべく三等分する問題。パターンの見落としがないかが勝負の分かれ目
- D問題 ... 数列から数を抜いて、前半と後半の差を大きくする問題。まず、全探索から一つのループで探索できないかアイデアが出るかの勝負。また、その後の実装も気をつけないと難しい。forの中ではlogNの最小のものが取り出せるheapqを使うっていう選択肢が取れるか。また、コストの管理にsumとか使うとfor合わせてN^2で破綻するので、forの中で定数時間で書き換えられる処理はないか気を配る必要がある。

#### keyence2019
- D問題 ... まず問題文を読んで場合分け及び解法を思いつくことができるかが分かれ目。その次にいかに計算量を削減できるかも重要。むしろ後者のほうが難しくて無限に時間を溶かした（C++なら平気だった？）

#### ABC117
- D問題 ... 初めてのビット演算。各桁が独立に決定できることを見抜けるかが決め手の第一歩。その次にアイデアを実装できるか。条件を満たしながら探索できるか。等学ぶことの多い問題。pythonでビットの処理や桁DPができるように練習を積む必要を感じた。

#### ABC118
- D問題 ... はじめてのDP。ちょうどN本マッチを使ってというところでどうやって最大桁数を探索すればよいのかわからなかった。思考が貪欲的な方法にとらわれていた。ちょうどN本ではなくちょうど0本だったら？1本だったら？i本だったら？と小さい方から順に決定できることに気づくのが肝心な問題。


#### ABC119
- C問題 ... 全探索にすぐ気づいたにもかかわらず、時間切れで解けなかった悔しい問題。どう探索したらバグが少なく済みそうか、楽に実装できそうか考えないで実装し始めて時間内にバグ取りが終わらないなんてほんとアホだよお前は。コンテストへの汎化能力を得るためにC_correct.pyでは深さ優先探索を用いている。深さ優先探索の際には参考になるだろう。

- D問題 ... 正直C問題より簡単だが、初めて二分探索を使ったのでコンテストで類問が出たときに使えるようにしておこう。 pythonではbisectを使うと良い。また大量に入力があるときはsys.stdin.readlineを用いると若干高速化される。

#### ABC120
- D問題 ... まさにアルゴリズム力が不足してるがために解けなかった問題。ノードのまとまりを管理する系の問題はUnionFindと覚えよう。

#### ABC122
- D問題 ... DP力不足。小さい状態から大きい状態が決まる系の問題はDPを疑う癖をつけよう。補集合とO(1)解法にこだわっていたのも良くない。D_correctは初期化がテクニカルでDPも多次元だが、よく考えればそんなに難しくはないので復習しよう。

#### ABC123
- D問題 ... 全探索の計算量をいかに削減するかが重要な問題。制約条件から気が付きたい。またヒープを使う爆速なやり方もあり、ヒープを使う場合は確認しておくたい。上位k位が気になるというのはだいたいヒープを使えば良いことが多い。


