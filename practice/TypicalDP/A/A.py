# https://atcoder.jp/contests/tdpc/tasks/tdpc_contest
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち

import numpy as np


def a_int(): return int(readline())


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


'''
dp[i, j] ... p[0:i]をまででj点を作ることができるか 1ができる 0ができない

dp[i+1,j] = dp[i, j-p[i]] | dp[i,j]
すでに作れれば必ず1だし、新たに作れれば+1にする
'''

N = a_int()
P = ints()

from numba import njit


@njit('(i8, i8[:])', cache=True)
def solve(N, P):
    S = P.sum()
    dp = np.zeros((N + 1, S + 1), dtype=np.int64)
    dp[0, 0] = 1
    for i in range(N):
        for j in range(S + 1):
            dp[i + 1, j] = dp[i, j]
            if j - P[i] >= 0:
                dp[i + 1, j] |= dp[i, j - P[i]]
    print(dp[N, :].sum())


solve(N, P)
