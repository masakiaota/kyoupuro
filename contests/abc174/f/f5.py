# 解説の解法はAtCoder expressみたいな二次元累積和っぽい考え方を平面走査とBIT(セグ木)で高速化
# ここではとりあえず遅延セグ木を用いてけんちょんさんの解法を投げてみる https://drken1215.hatenablog.com/entry/2019/01/01/234400
# 遅延セグ木がTLEしたので,BITで...


class BIT:
    def __init__(self, n):
        self.n = n
        self.bit = [0] * (self.n + 1)  # bitは(1based indexっぽい感じなので)

    def init(self, ls):
        assert len(ls) <= self.n
        # lsをbitの配列に入れる
        for i, x in enumerate(ls):  # O(n log n 掛かりそう)
            self.add(i, x)

    def add(self, i, x):
        '''i番目の要素にxを足し込む'''
        i += 1  # 1 based idxに直す
        while i <= self.n:
            self.bit[i] += x
            i += (i & -i)

    def sum(self, i, j):
        '''[i,j)の区間の合計を取得'''
        return self._sum(j) - self._sum(i)

    def _sum(self, i):
        '''[,i)の合計を取得'''
        # 半開区間なので i+=1しなくていい
        ret = 0
        while i > 0:
            ret += self.bit[i]
            i -= (i & -i)
        return ret


class RangeAddBIT:  # range add query
    # [l,r)にxを加算する
    # [0,[l,r), i)のとき→bit.sum(i)+(rx-lx) (iによらない)
    # [0,[l,i),r)のとき→bit.sum(i)+(ix-lx)
    # [0,i),[l,r)のとき→bit.sum(i) (iによらない)
    # を加算できれば良い。bit.sum(i)が0だと見立てるとわかりやすいかも？
    # 場合分け2つ目における1項目をbit1,2項目をbit2とする
    def __init__(self, n: int):
        self.n = n
        self.bit1 = BIT(n)  # bit1.sum(i)*iで xiを達成したい部分 imos方的に使う
        self.bit2 = BIT(n)  # bit2.sum(i)で -xlを達成したい部分が手に入る。 r<iで区間加算後の和に相当する

    def add(self, l: int, r: int, x):
        '''[l,r)の要素にxを足し込む'''
        self.bit1.add(l, x)
        self.bit1.add(r, -x)
        self.bit2.add(l, -x * l)
        self.bit2.add(r, x * r)

    def sum(self, l, r):
        '''[l,r)の区間の合計を取得'''
        return self._sum(r) - self._sum(l)

    def _sum(self, i: int):
        '''[,i)の合計を取得'''
        return self.bit1._sum(i) * i + self.bit2._sum(i)


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
segtree = RangeAddBIT(N + 10)  # l-rまでの種類数を格納(rを広げながら更新していく)
idxprev = defaultdict(lambda: -1)  # 各色の前に出現したidx [prevの位置+1,再出現の位置+1)に区間加算する

ans = [-1] * Q
r_now = 0  # 現在の考慮する位置
for l, r, i in LRI:
    for j in range(r_now, r):
        prev = idxprev[C[j]]
        segtree.add(prev + 1, j + 1, 1)
        idxprev[C[j]] = j
        r_now = j
    ans[i] = segtree.sum(l, l + 1)
print(*ans, sep='\n')

# TLEなんですけど...
