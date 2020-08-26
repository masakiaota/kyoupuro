# https://atcoder.jp/contests/dp/tasks/dp_l

'''
dp[i,j] ... dequeに[i,j]が残っているときに行動するプレイヤーのスコアの最大
dp[i,j] = max(a[i-1]-dp[i-1,j], a[j+1]-dp[i,j+1]) #これじゃgreedyと同じじゃん

dp[i,j] ... dequeに[i,j)が残っているときにその状態からスタートするプレイヤーが達成できるスコアの最大 (区間を伸ばしていく感じ)

初期値 dp[i,i]==0

dp[i,j]=max(a[i]-dp[i+1,j], a[j-1]-dp[i,j-1])
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


def a_int(): return int(readline())


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import

N = a_int()
A = ints()

dp = zeros((N + 1, N + 1))

from numba import njit


@njit('(i8,i8[:],i8[:,:])', cache=True)
def solve(N, A, dp):
    for i in range(N - 1, -1, -1):
        for j in range(i + 1, N + 1):
            dp[i, j] = max(A[i] - dp[i + 1, j], A[j - 1] - dp[i, j - 1])
    print(dp[0, N])
    # print(*dp, sep='\n')


solve(N, A, dp)
