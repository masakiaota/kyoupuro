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


def a_int(): return int(read())


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import


K = np.array(list(map(int, list(input()))))
D = a_int()

'''
ある数xにおいて総和がDの倍数であるかの判定
x[0]+x[1]...+x[n] ≡ 0 (mod D) 各桁の和にMODを取れば良い。つまり桁ごとに処理できる

dp[i,j,k] ... K[:i]以下の整数のうち、k=mod Dとなるような数字の個数。j==1は上位桁が一致している場合の通りの個数。j==0は上位桁が一致しなくなったときの個数。

遷移
for l in range(10):
    dp[i+1,0,(k+l)%D] += dp[i,0,k] #1~9を考慮可能
for l in range(K[i]):
    dp[i+1,0,(k+l)%D] += dp[i,1,k] #1~K[i]-1をつけるとぴったりにならないグループへ行く

dp[i+1,1,(k+K[i])%D] += dp[i,1,k]
'''
# print(K, D)

from numba import njit


@njit('(i8[:],i8)', cache=True)
def solve(K, D):
    N = K.shape[0]
    dp = np.zeros((N + 1, 2, D), dtype=np.int64)
    dp[0, 1, 0] = 1
    for i in range(N):  # 各桁について
        for k in range(D):  # 各あまりについて処理
            # for l in range(K[i]):
            #     # 1~K[i]-1をつけるとぴったりにならないグループへ行く
            #     dp[i + 1, 0, (k + l) % D] += dp[i, 1, k]
            # for l in range(10):  # 各数字について処理
            #     dp[i + 1, 0, (k + l) % D] += dp[i, 0, k]  # 1~9を考慮可能
            #     dp[i + 1, 0, (k + l) % D] %= MOD
            for l in range(10):
                m = (k + l) % D
                if l < K[i]:
                    dp[i + 1, 0, m] += dp[i, 0, k] + dp[i, 1, k]
                else:
                    dp[i + 1, 0, m] += dp[i, 0, k]  # 1~9を考慮可能
                dp[i + 1, 0, m] %= MOD

            # ぴったりグループ
            dp[i + 1, 1, (k + K[i]) % D] += dp[i, 1, k]
            # dp[i + 1, 1, (k + K[i]) % D] %= MOD

    print((dp[-1, :, 0].sum() - 1) % MOD)  # -1は0の分


solve(K, D)
