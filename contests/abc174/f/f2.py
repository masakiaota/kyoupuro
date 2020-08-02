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


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


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


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


from bisect import bisect_left, bisect_right, insort_left

from functools import reduce


class SqrtDecomposedList:
    def __init__(self, ls, segfunc, identity_element):
        from math import sqrt, ceil
        from copy import copy
        self.f = segfunc  # queryのfunction
        self.ide = identity_element
        self.n = len(ls)
        self.ls = copy(ls)
        self.b = ceil(sqrt(self.n))
        self.bucket = [self.ide] * (self.b + 1)
        self._build()

    def _build(self):
        for d in range(self.b):
            self.bucket[d] = reduce(
                self.f, self.ls[(d * self.b):((d + 1) * self.b)], self.ide)

    def __getitem__(self, idx):
        return self.ls[idx]

    def update(self, i, x):
        '''i番目の要素をxに更新する O(√n)'''
        self.ls[i] = x
        s = i // self.b * self.b
        t = s + self.b
        # print(i, self.b, i // self.b, s, t)
        self.bucket[i // self.b] = reduce(self.f,
                                          self.ls[s:t], self.ide)

    def query(self, l, r):
        '''半開区間[l,r)で指定すること O(√n)'''
        if r - l < 2 * self.b:
            return reduce(self.f, self.ls[l:r])
        l_bucket = (l - 1) // self.b + 1
        r_bucket = r // self.b  # 半開区間なのでこれでいい
        ret = reduce(self.f, self.bucket[l_bucket:r_bucket], self.ide)
        ret = self.f(
            reduce(self.f, self.ls[l:(l_bucket * self.b)], self.ide),
            ret)
        ret = self.f(
            reduce(self.f, self.ls[(r_bucket * self.b):r], self.ide),
            ret)
        return ret


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.func = segfunc
        self.n_origin = len(ls)
        self.num = 2 ** (self.n_origin - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def __getitem__(self, idx):  # オリジナル要素にアクセスするためのもの
        if isinstance(idx, slice):
            start = idx.start if idx.start else 0
            stop = idx.stop if idx.stop else self.n_origin
            l = start + self.num - 1
            r = l + stop - start
            return self.tree[l:r:idx.step]
        elif isinstance(idx, int):
            i = idx + self.num - 1
            return self.tree[i]

    def update(self, i, x):
        '''
        i番目の要素をxに変更する(木の中間ノードも更新する) O(logN)
        '''
        i += self.num - 1
        self.tree[i] = x
        while i:  # 木を更新
            i = (i - 1) // 2
            self.tree[i] = self.func(self.tree[i * 2 + 1],
                                     self.tree[i * 2 + 2])

    def query(self, l, r):
        '''区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど'''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        res_right = []
        res_left = []
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if l & 1:
                res_left.append(self.tree[l - 1])
                l += 1
            if r & 1:
                r -= 1
                res_right.append(self.tree[r - 1])
            l >>= 1
            r >>= 1
        res = self.ide
        # 左右の順序を保って結合
        for x in res_left:
            res = self.func(x, res)
        for x in reversed(res_right):
            res = self.func(res, x)
        return res


def segfunc(x, y):  # セット
    return x | y


N, Q = ints()
C = []
for c in ints():
    C.append({c})

ls = SegmentTree(C, segfunc, set())
# セグ木か？面倒なので平方分割で
ans = []
for _ in range(Q):
    l, r = ints()
    ans.append(len(ls.query(l - 1, r)))
print(*ans, sep='\n')
