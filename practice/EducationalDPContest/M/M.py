'''
問題を言い換えると
0<=bi<=ai for any iにおいて、sum_i bi = Kとなる通りの数を求めよ

dp[i,j] .. 通りの総数、総和がちょうどjになるとき、:iまでの子供を考慮したとき
dp[i+1,j]=sum_{k=0}^{a_{i}} dp[i,j-k]

これではTLEになりそうなので、累積和で高速に計算する必要がある(ところてん方式でやった)
'''

import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち

import numpy as np
from functools import partial
array = partial(np.array, dtype=np.int64)
zeros = partial(np.zeros, dtype=np.int64)
full = partial(np.full, dtype=np.int64)


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


MOD = 10**9 + 7

N, K = ints()
A = ints()

from numba import njit


@njit('(i8,i8,i8[:])', cache=True)
def solve(N, K, A):
    dp = np.zeros((N + 1, K + 1), dtype=np.int64)
    dp[0, 0] = 1
    for i in range(N):
        # 前の列の累積和を作りたい
        total = 0
        for j in range(K + 1):
            # ai個でところてんすればいい
            total += dp[i, j]
            total %= MOD
            if j - A[i] - 1 >= 0:
                total -= dp[i, j - A[i] - 1]
            dp[i + 1, j] = total % MOD

    print(dp[N, K])


solve(N, K, A)
