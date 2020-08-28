import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち

import numpy as np
from functools import partial
array = partial(np.array, dtype=np.int64)
zeros = partial(np.zeros, dtype=np.int64)
full = partial(np.full, dtype=np.int64)

ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

'''
DPじゃなくて貪欲に小さい方からくっつけるのじゃだめ？←隣り合うという制約が入れられない

sum a_i から をaiに分解するイメージ
dp[i,j] ... コストの最小. A[i:j]を合体させたとき

更新式
dp[i,j] = min(dp[i,i+1] + dp[i+1,j],
              dp[i,i+2] + dp[i+2,j],...
              ) + sum(A[i,j])

dp[i,i+1]=0かな？
'''


N = a_int()
A = ints()
A_accum = zeros(N + 1)
A_accum[1:] = np.cumsum(A)
dp = full((N + 1, N + 1), -1)
dp[np.arange(N), np.arange(N) + 1] = 0

from numba import njit


@njit('(i8,i8,i8[:,:],i8[:])', cache=True)
def dfs(i, j, dp, A_accum):
    if dp[i, j] != -1:
        return dp[i, j]
    mi = 10**14
    for k in range(i + 1, j):
        mi = min(mi, dfs(i, k, dp, A_accum) + dfs(k, j, dp, A_accum))
    dp[i, j] = mi + A_accum[j] - A_accum[i]
    return dp[i, j]


print(dfs(0, N, dp, A_accum))
