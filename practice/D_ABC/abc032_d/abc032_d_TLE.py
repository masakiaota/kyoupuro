# https://atcoder.jp/contests/abc032/tasks/abc032_d

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N, K = read_ints()
V, W = read_col(N)

# 愚直dp[i][w]は間に合わない
# 1. 再帰メモ化で実装するとどう？←TLE
# 2. もしくはdp[i][v]...[0,i)考慮したときに価値vになる最小の重さをやるのは？←vもクソでかい
# 3, 制約をうまく使う→制約はNが小さいかwが小さいかvが小さいと言っている 小さいのを最小する実装をすれば良い


# 楽だから一回再帰メモ化で実装してみるか←本質的な計算量改善ではないのでTLEになった
memo = defaultdict(lambda: None)


def f(i, w):
    if memo[i, w] != None:
        return memo[i, w]
    if i == 0 or w == 0:
        return 0

    item_no = f(i - 1, w)
    if w - W[i - 1] < 0:
        item_in = 0
    else:
        item_in = f(i - 1, w - W[i - 1]) + V[i - 1]

    memo[i, w] = max(item_no, item_in)
    return memo[i, w]


print(f(N, K))
