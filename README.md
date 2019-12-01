競プロをやる
===

### コピペ用
テンプレ
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
    return [read()[:-1] for _ in range(H)]

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
```


```python
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

# グラフ構造が与えられたとき
from collections import defaultdict
node = defaultdict(set)  # キーで指定したノードが隣接するノードを記録しておく。
for _ in range(与えられる列数):
    u, v, w = read_ints() # assume 2 node idx and edge weight are given
    node[v-1].add((u-1, w))
    node[u-1].add((v-1, w))


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
A_accum=[0]+list(accumelate(A))
# Aの累積和のリストがこれでできる
# [0]は半開区間できれいに指定するため
#A      :  1 2 3 4
#A_accum: 0 1 3 6 10
# A[1:3]の累積和(つまり2+3)はA_accum[3]-A_accum[1]で済む

# 最小公倍数を求める
from fractions import gcd
#gcd(0,3)とか3と答える便利な性質がある。3だけの場合の最大公約数(つまり3)を求めることができるということ.

# i番目の要素でソート
# hoge=[(1,4),(2,3),(3,2),(4,1)]みたいなやつを`1`番目の要素でソートしたい
from operator import itemgetter
hoge.sort(key=itemgetter(1))
# [(4, 1), (3, 2), (2, 3), (1, 4)]

# mod取りながらcombination
def combination_mod(n, r, mod):
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod

```

その他コピペで便利そうなアルゴリズムは`algorithm/`に保存してある。

### 手法実装例へのリンク
- [深さ優先探索](https://github.com/masakiaota/kyoupuro/blob/master/practice/novice/ARC031_B.py)
  - [深さ優先探索を用いた閉路判定](https://github.com/masakiaota/kyoupuro/blob/master/practice/novice/ARC037_B.py)
- [再帰全探索メモ化](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/142_ALDS1_5_A.py)
- [幅優先探索]( https://github.com/masakiaota/kyoupuro/blob/master/algorithm/BFS.py )
- [UnionFind木](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/UnionFind.py)
  - [具体的に問題でつかってのはこれ](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC120/D_correct.py)
- [ビット全探索](https://github.com/masakiaota/kyoupuro/blob/master/practice/novice/ABC045_C.py)
- [二分探索]( https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC119/D_correct.py )
  - [二分探索木](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/BinarySearchTree.py)
  - 単調増加関数をかませるパターンの二分探索(ライブラリが使えない) [螺旋本](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/136_ALDS1_4_D.py) [ABC146](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC146/C.py)
- [priority queue (コピペ用)](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/PriorityQueue.py)
  - [実際に使われた問題](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC123/D_correct5.py)
- ソート関係
  - [Counting Sort](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/168_ALDS1_6_A.py) (出ないかも)
  - バブルソートをするとしたら[要素の反転数は？](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/175_ALDS1_5_D.py) O(n**2)より高速なO(nlogn)
  - [最小コストソート](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/179_ALDS1_6_D.py)...最小コストで要素をswapさせてソートしたい
- DP関係
  - [共通部分文字列](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/253_ALDS1_10_C.py)
  - [連鎖行列積](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/257_ALDS1_10_B.py)
- scipy関係
  - [Minimum Spanning Tree](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/296_ALDS1_12_A_AtCoder.py) データを打ち込むときに疎行列を使った[バージョン](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/358_GRL_2_A_AtCoder.py)
<!--  TODO ダイクストラ法から  -->


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
- D問題 ... 全探索の計算量をいかに削減するかが重要な問題。制約条件から気が付きたい。またPriorityQueueを使う爆速なやり方もあり、これを使う場合は確認しておくたい。上位k位が気になるというのはだいたいPriorityQueueを使えば良いことが多い。(本番で一瞬頭にちらついたのに…)

#### ABC124
- D問題 ... データ構造を工夫して、累積和かしゃくとり法を使う問題。解法までわかったのに実装のバグが取れずに非常に悔しい思いをした(そして今もバグが取れていない)。解き方の大枠だけでなく、データ構造を工夫する(連長圧縮というらしい)ことで楽に実装できないかまでちゃんと考えよう。(でもいままでD問題は見当すらつかなかったのに最近は解法まで思いついているのは圧倒的成長だよ自分)

#### ABC125
- C問題 ... 累積和は差分だけど、累積しておく概念はそれにとどまらない。この問題では累積GCDで計算量を削減しているが、同じような考え方で累積xorと累積積とかできる。

#### AGC032
- A問題 ... きちんと想定解法を思い浮かんだのにTLEが取れなかった悔しい問題(Pythonが遅いからってのもあるが)。何かが存在する（しない）を確かめるときは、その何かの数を持っておいて更新していく方式の方が圧倒的に早い。



