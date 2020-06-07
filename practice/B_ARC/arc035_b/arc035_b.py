# https://atcoder.jp/contests/arc035/tasks/arc035_b
# ソートしてから同じ要素の個数をP!して和を取っていく
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def perm_mod(n, r, mod=10**9 + 7):
    '''nPrをmodを取って返す'''
    if n < r:  # そんな通りはありえない
        return 0
    ret = 1
    for _ in range(r):
        ret *= n
        ret %= mod
        n -= 1
    return ret


from collections import Counter
from itertools import accumulate
MOD = 10 ** 9 + 7
N = read_a_int()
T, = read_col(N)
T.sort()
cnt = Counter(T)

ans1 = sum(accumulate(T))
ans2 = 1
for v in cnt.values():
    ans2 *= perm_mod(v, v)
    ans2 %= MOD
print(ans1)
print(ans2)
