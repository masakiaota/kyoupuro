競プロをやる
===

### コピペ用
テンプレ
```python
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra=range
enu=enumerate

def read_ints():
    return list(map(int, read().split()))

def read_a_int():
    return int(read())

def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret

def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))

def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため

def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]

def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret

MOD = 10**9 + 7
INF=2**31 # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right #, insort_left, insort_right
from math import gcd

def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a * b // g
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

# 文字の順番を扱うとき
def ord_from_a(char):
    return ord(char) - ord('a')
def chr_from_a(n: int):
    # nはaから何番あとかを示す
    return chr(n + ord('a'))

# グラフ構造が与えられたとき
from collections import defaultdict
node = defaultdict(set)  # キーで指定したノードが隣接するノードを記録しておく。
for _ in range(与えられる列数):
    u, v, w = read_ints() # assume 2 node idx and edge weight are given
    node[v-1].add((u-1, w))
    node[u-1].add((v-1, w))


#再帰メモ化
from functools import lru_cache
@lru_cache(maxsize=2**12)

#要素の数え上げ(文字数の数え上げに便利
from collections import Counter
dic = Counter(なんかの配列)

# 最小公倍数を求める
from fractions import gcd
#gcd(0,3)とか3と答える便利な性質がある。3だけの場合の最大公約数(つまり3)を求めることができるということ.


# i番目の要素でソート
# hoge=[(1,4),(2,3),(3,2),(4,1)]みたいなやつを`1`番目の要素でソートしたい
from operator import itemgetter
hoge.sort(key=itemgetter(1))
# [(4, 1), (3, 2), (2, 3), (1, 4)]

# mより小さい最大値 (最小値も似たようにかける)
max(arr, key=lambda x: -INF if x>m  else x)

# mod取りながらcombination
def combination_mod(n, r, mod):
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod

# np.full と同等のpython実装
def full(shape, full_value):
    if isinstance(shape, tuple):
        sha = shape[::-1]
        ret = [full_value] * sha[0]
        for s in sha[1:]:
            ret = [ret.copy() for i in range(s)] #ここ、pypyだと何故かバグが出る
        return ret
    else:
        return [full_value] * shape
```

その他コピペで便利そうなアルゴリズムは`algorithm/`に保存してある。

### 手法実装例へのリンク
- [UnionFind木](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/UnionFind.py)
  - [具体的に問題でつかってのはこれ](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC120/D_correct.py)
- [ビット全探索](https://github.com/masakiaota/kyoupuro/blob/master/practice/novice/ABC045_C.py) [ほかにはこれ](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc128_c.py) [1立つidxだけ考慮したりするやつ](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc147_c.py)
    - p進数全探索 [テンプレート](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/iter_p_adic.py)

- [Segment Tree](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/SegmentTree.py)
  - [例題](https://github.com/masakiaota/kyoupuro/blob/master/practice/E_ABC/abc157_e.py)

- 数列系
  - [累積和](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/cumsum.py) [二次元累積和問題](https://github.com/masakiaota/kyoupuro/blob/master/practice/D_ABC/abc106_d.py)
  - 累積xor [■](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/cumsum.py)
  - 尺取法 [■](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/cumsum.py)
  - [imos法](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/cumsum.py) パルス波をデルタ関数の積分で表現するイメージ
    - 前から積分しながら、先に負のデルタ関数の予約するimos [■](https://github.com/masakiaota/kyoupuro/tree/master/practice/F_ABC)
    - imosした結果の最大値最小値を使う問題 [■](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc017_c.py)
  - 座標圧縮 with 二次元累積和 の問題 [■](https://github.com/masakiaota/kyoupuro/blob/master/practice/D_ABC/abc075_d.py)

- [二分探索]( https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC119/D_correct.py )
  - [二分探索木](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/BinarySearchTree.py)
  - 単調増加関数をかませるパターンの二分探索(ライブラリが使えない) [螺旋本](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/136_ALDS1_4_D.py) [ABC146](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC146/C.py)
  - めぐる式[二分探索](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/meguru_bisect.py) [ABC146](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc146_c.py)
  - bisect for reversed list [■](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/bisect_for_reversedlist.py)

- [priority queue (コピペ用)](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/PriorityQueue.py)
  - [実際に使われた問題](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC123/D_correct5.py)

- ソート関係
  - [Counting Sort](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/168_ALDS1_6_A.py) (出ないかも)
  - バブルソートをするとしたら[要素の反転数は？](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/175_ALDS1_5_D.py) O(n**2)より高速なO(nlogn)
  - [最小コストソート](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/179_ALDS1_6_D.py)...最小コストで要素をswapさせてソートしたい
- DP関係
  - [共通部分文字列](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/253_ALDS1_10_C.py)
  - [連鎖行列積](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/257_ALDS1_10_B.py)
  - [最大長方形](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/428_DPL_3_B.py)
  - [コイン問題](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/412_DPL_1_A.py)
  - [ナップサックの復元](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/416_DPL_1_B_items.py)
  - 桁DP [再帰実装1](https://github.com/masakiaota/kyoupuro/blob/master/practice/D_ABC/abc007_d.py) [2](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC154/E_correct.py) [配列実装1](https://github.com/masakiaota/kyoupuro/blob/master/practice/D_ABC/abc007_d.py) [2](https://github.com/masakiaota/kyoupuro/blob/master/contests/ABC154/E_correct2.py)
  - 数列から選んだk個選んだときに合計がsになる通りの数のDP[■](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc044_c.py)
  - 数列から0つ以上選んだとき合計がtになる通りの数のDP (tは負を許す) [■](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc044_c.py)
    - これの応用で重さが負のナップサックも解けそう。

- グラフ関係
  - [深さ優先探索](https://github.com/masakiaota/kyoupuro/blob/master/practice/novice/ARC031_B.py)
    - [深さ優先探索を用いた閉路判定](https://github.com/masakiaota/kyoupuro/blob/master/practice/novice/ARC037_B.py)
  - [再帰全探索メモ化](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/142_ALDS1_5_A.py)
  - [幅優先探索]( https://github.com/masakiaota/kyoupuro/blob/master/algorithm/BFS.py)
  - [トポロジカルソート](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/342_GRL_4_B.py)
  - [木の直径](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/353_GRL_5_A.py)
  - [lowest link](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/lowest_link.py)
    - [関節点](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/348_GRL_3_A.py)の例
    - [橋](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc075_c.py)の例

- scipy.csgraph 関係 [■](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/scipy_csgraph.py)
  - [dijkstra法](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/309_ALDS1_12_C_AtCoder.py)
    - dijkstraを用いた経路復元 [■](https://github.com/masakiaota/kyoupuro/blob/master/practice/D_ABC/abc051_d/abc051_d.py)
  - [warshall floyd](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/336_GRL_1_C_AtCoder.py)
  - [Minimum Spanning Tree](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/296_ALDS1_12_A_AtCoder.py) データを打ち込むときに疎行列を使った[バージョン](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/358_GRL_2_A_AtCoder.py)

- [2D Tree](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/324_DSL2_C.py)

- 計算幾何
  - [線分交差判定](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/387_CGL_2_B.py)
  - [点の内包判定](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/398_CGL_3_C.py)
  - [二次元凸包作成](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/401_CGL_4_A.py)
  - [マンハッタン線分交差高速判定](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/405_CGL_6_A.py)

- 整数論
  - [素数判定](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/436_ALDS_1_C.py) [エラトステネスの篩](https://github.com/masakiaota/kyoupuro/blob/master/practice/algorithm_datastructure_for_programming_contest/436_ALDS_1_C_eratosthenes.py)
  - [約数全列挙](https://github.com/masakiaota/kyoupuro/blob/master/practice/D_ABC/abc112_d.py)の問題
  - ココらへんの[スニペット](https://github.com/masakiaota/kyoupuro/blob/master/algorithm/Prime_numbers.py)

- 誤差関係
  - Decimalを利用した誤差なし四捨五入 [■](https://github.com/masakiaota/kyoupuro/blob/master/practice/C_ABC/abc001_c.py)

### TODO
- 耳DPの習得 (これはよく使いそう？)
- scipy_csgraph.pyの整備 (最短全域木とか)
- 木構造クラスの整備(共通ルート高速取得とかまだ)
- 挿入などが高速に行えるデータ構造の履修(Treapとか)

### あとで解き直したい
- ABC018 C [菱形カウント](https://atcoder.jp/contests/abc018/tasks/abc018_3) ... 多点からの最小マンハッタン距離を求めるDP

- ABC044 C [高橋くんとカード](https://atcoder.jp/contests/abc044/tasks/arc060_a) ... 典型的なDPっぽい

- 住友銀行 E [colorful hat 2](https://atcoder.jp/contests/sumitrust2019/tasks/sumitb2019_e) ... 通りの数を独立に扱いたい。考慮できる情報はわからないか？

- diverta C [AB Substrings](https://atcoder.jp/contests/diverta2019/tasks/diverta2019_c) ... コーナーケース見落とし

- AGC036 A [Triangle](https://atcoder.jp/contests/agc036/tasks/agc036_a) ... 天才パズルすぎる.(外積、剰余計算)

- ABC150 D [Semi Common Multiple](https://atcoder.jp/contests/abc150/tasks/abc150_d) ... 必要条件見落とさない！

- ABC113 D [Number of Amidakuji](https://atcoder.jp/contests/abc113/tasks/abc113_d) ... 面白いDP。実装が重め。もう一度解きたい(今度はすばやく実装したい)

- ABC074 D [Restoring Road Network](https://atcoder.jp/contests/abc074/tasks/arc083_b) ... ちょっと変わったグラフの問題。水diffだけど難しい。



水色diffは解き直してみてもいいかもしれない。

#### BeginnersSelectioin/novice
- ABC045_C問題 ... ビット全探索。考え方は難しくないが、ビット全探索という操作の書き方はなれておくと便利そう。また文字列をpythonスクリプトとして実行することもしたので必要になったときは見るべし。


#### ABC070
- D問題 ... 木構造の探索の問題。実装では深さ優先探索を採用した。

#### ABC062
- C問題 ... 長方形にたいして長方形を使ってなるべく三等分する問題。パターンの見落としがないかが勝負の分かれ目
- D問題 ... 数列から数を抜いて、前半と後半の差を大きくする問題。まず、全探索から一つのループで探索できないかアイデアが出るかの勝負。また、その後の実装も気をつけないと難しい。forの中ではlogNの最小のものが取り出せるheapqを使うっていう選択肢が取れるか。また、コストの管理にsumとか使うとfor合わせてN^2で破綻するので、forの中で定数時間で書き換えられる処理はないか気を配る必要がある。

#### keyence2019
- D問題 ... まず問題文を読んで場合分け及び解法を思いつくことができるかが分かれ目。その次にいかに計算量を削減できるかも重要。むしろ後者のほうが難しくて無限に時間を溶かした（C++なら平気だった？）

#### ABC117
- D問題 ... 初めてのビット演算。各桁が独立に決定できることを見抜けるかが決め手の第一歩。その次にアイデアを実装できるか。条件を満たしながら探索できるか。等学ぶことの多い問題。pythonでビットの処理や桁DPができるように練習を積む必要を感じた。

#### ABC119
- C問題 ... 全探索にすぐ気づいたにもかかわらず、時間切れで解けなかった悔しい問題。どう探索したらバグが少なく済みそうか、楽に実装できそうか考えないで実装し始めて時間内にバグ取りが終わらないなんてほんとアホだよお前は。コンテストへの汎化能力を得るためにC_correct.pyでは深さ優先探索を用いている。深さ優先探索の際には参考になるだろう。

- D問題 ... 正直C問題より簡単だが、初めて二分探索を使ったのでコンテストで類問が出たときに使えるようにしておこう。 pythonではbisectを使うと良い。また大量に入力があるときはsys.stdin.readlineを用いると若干高速化される。

#### ABC122
- D問題 ... DP力不足。小さい状態から大きい状態が決まる系の問題はDPを疑う癖をつけよう。補集合とO(1)解法にこだわっていたのも良くない。D_correctは初期化がテクニカルでDPも多次元だが、よく考えればそんなに難しくはないので復習しよう。

#### ABC123
- D問題 ... 全探索の計算量をいかに削減するかが重要な問題。制約条件から気が付きたい。またPriorityQueueを使う爆速なやり方もあり、これを使う場合は確認しておくたい。上位k位が気になるというのはだいたいPriorityQueueを使えば良いことが多い。(本番で一瞬頭にちらついたのに…)

#### ABC124
- D問題 ... データ構造を工夫して、累積和かしゃくとり法を使う問題。解法までわかったのに実装のバグが取れずに非常に悔しい思いをした(そして今もバグが取れていない)。解き方の大枠だけでなく、データ構造を工夫する(連長圧縮というらしい)ことで楽に実装できないかまでちゃんと考えよう。(でもいままでD問題は見当すらつかなかったのに最近は解法まで思いついているのは圧倒的成長だよ自分)

#### AGC032
- A問題 ... きちんと想定解法を思い浮かんだのにTLEが取れなかった悔しい問題(Pythonが遅いからってのもあるが)。何かが存在する（しない）を確かめるときは、その何かの数を持っておいて更新していく方式の方が圧倒的に早い。
