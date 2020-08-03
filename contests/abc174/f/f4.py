# 解説の解法はAtCoder expressみたいな二次元累積和っぽい考え方を平面走査とBIT(セグ木)で高速化
# ここではとりあえず遅延セグ木を用いてけんちょんさんの解法を投げてみる https://drken1215.hatenablog.com/entry/2019/01/01/234400


def _gidx(l, r, treesize):
    '''
    lazy propagation用idx生成器 木の下から生成される。1based-indexなので注意.(使うときは-1するとか)
    もとの配列において[l,r)を指定したときに更新すべきidxをyieldする
    treesizeは多くの場合self.num
    '''
    L, R = l + treesize, r + treesize
    lm = (L // (L & -L)) >> 1  # これで成り立つの天才か？
    rm = (R // (R & -R)) >> 1
    while L < R:
        if R <= rm:
            yield R
        if L <= lm:
            yield L
        L >>= 1
        R >>= 1
    while L:  # Rでもいいけどね
        yield L
        L >>= 1


import operator


class SegmentTreeForRSQandRAQ:  # 区間合計(ホントは何でも良い)クエリ と 区間加算クエリを扱うことにする
    def __init__(self, ls: list, segfunc=operator.add, identity_element=0, lazy_ide=0):
        '''
        セグ木 もしかしたらバグがあるかも
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.lide = lazy_ide  # lazy用単位元
        self.func = segfunc
        n = len(ls)
        self.num = 2 ** (n - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num)  # 関係ない値を-1においてアクセスを許すと都合が良い
        self.lazy = [self.lide] * (2 * self.num)  # 遅延配列
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def _lazyprop(self, *ids):
        '''
        遅延評価用の関数
        - self.tree[i] に self.lazy[i]の値を伝播させて遅延更新する
        - 子ノードにself.lazyの値を伝播させる **ここは問題ごとに書き換える必要がある**
        - self.lazy[i]をリセットする
        '''
        for i in reversed(ids):
            i -= 1  # to 0basedindex
            v = self.lazy[i]
            if v == self.lide:
                continue
            #########################################################
            # この4つの配列をどう更新するかは問題によって異なる
            self.tree[2 * i + 1] += v >> 1  # 区間加算クエリなので
            self.tree[2 * i + 2] += v >> 1
            self.lazy[2 * i + 1] += v >> 1
            self.lazy[2 * i + 2] += v >> 1
            #########################################################

            self.lazy[i] = self.lide  # 遅延配列を空に戻す

    def update(self, l, r, x):
        '''
        [l,r)番目の要素をxを適応するクエリを行う O(logN)
        '''
        # 1, 根から区間内においてlazyの値を伝播しておく(self.treeの値が有効になる)
        ids = tuple(_gidx(l, r, self.num))
        #########################################################
        # 区間加算queryのような操作の順序が入れ替え可能な場合これをする必要なないが多くの場合でしたほうがバグが少なく(若干遅くなる)
        # self._lazyprop(*ids)
        #########################################################
        # 2, 区間に対してtree,lazyの値を更新 (treeは根方向に更新するため、lazyはpropで葉方向に更新するため)
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        while l < r:
            #########################################################
            # ** 問題によって値のセットの仕方も変えるべし**
            if r & 1:
                r -= 1
                self.tree[r - 1] += x
                self.lazy[r - 1] += x
            if l & 1:
                self.tree[l - 1] += x
                self.lazy[l - 1] += x
                l += 1
            #########################################################
            x <<= 1  # 区間加算クエリでは上段では倍倍になるはずだよね
            l >>= 1
            r >>= 1
        # 3, 伝播させた区間について下からdataの値を伝播する
        for i in ids:
            i -= 1  # to 0based
            #########################################################
            # 関数の先頭でlazy propを省略した場合は、現在のノードにlazyが反映されていないことがある
            # lazyを省略するならここを慎重に書き換えなければならない
            self.tree[i] = self.func(
                self.tree[2 * i + 1], self.tree[2 * i + 2]) + self.lazy[i]
            #########################################################

    def query(self, l, r):
        '''
        区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど
        '''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        # 1, 根からにlazyの値を伝播させる
        self._lazyprop(*_gidx(l, r, self.num))
        # 2, 区間[l,r)の最小値を求める
        l += self.num
        r += self.num
        res = self.ide
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if r & 1:
                r -= 1
                res = self.func(res, self.tree[r - 1])
            if l & 1:
                res = self.func(res, self.tree[l - 1])
                l += 1
            l >>= 1
            r >>= 1
        return res


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd
N, Q = ints()
C = ints()
L, R = read_col(Q)
LRI = [(l - 1, r, i) for i, (l, r) in enu(zip(L, R))]
LRI.sort(key=itemgetter(1))  # 右端でソートしておく
segtree = SegmentTreeForRSQandRAQ(
    [0] * (N + 10))  # l-rまでの種類数を格納(rを広げながら更新していく)
idxprev = defaultdict(lambda: -1)  # 各色の前に出現したidx [prevの位置+1,再出現の位置+1)に区間加算する

ans = [-1] * Q
r_now = 0  # 現在の考慮する位置
for l, r, i in LRI:
    while r_now < r:
        prev = idxprev[C[r_now]]
        segtree.update(prev + 1, r_now + 1, 1)
        idxprev[C[r_now]] = r_now
        r_now += 1
    ans[i] = segtree.query(l, l + 1)
print(*ans, sep='\n')
