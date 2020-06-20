# https://atcoder.jp/contests/agc046/tasks/agc046_b
#
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


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


def combination_mod(n, r, mod=998244353):
    # mod取りながらcombination
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


MOD = 998244353
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd

A, B, C, D = read_ints()

# a*bマスは必ず白
# 基本は(c*d-a*b)マスの中から(c-a)+(d-b)選ぶ通りの数(？)
# この選び方では隣接する行をすべて塗ることができてしまう
ans = 1
while A < C and B < D:
    n_side = (A + 1) * (B + 1) - A * B
    ans *= combination_mod(n_side, 2)
    ans %= MOD
    A += 1
    B += 1

while A < C:
    ans *= B
    ans %= MOD
    A += 1

while B < D:
    ans *= A
    ans %= MOD
    B += 1
print(ans)
