import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


import numpy as np
N, K = read_ints()
H = np.array(read_ints())
K = min(len(H), K)

dp = np.full((N), np.inf)
dp[0] = 0
# Kまで初期化しておかないと厄介なことになる
# i-kのコストは直接飛ぶのが必ず最小になる
dp[1:K+1] = np.abs(H[0]-H[1:K+1])

for i in range(K+1, N):
    # i-Kまでのコストリストを作成しておく
    dp[i] = (dp[(i - K):(i)] + np.abs(H[i] - H[(i - K):(i)])).min()


print(int(dp[-1]))


# 配るDPでもやってみたい
