# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/10/ALDS1_10_C
# どうあがいてもTLEのカルマから逃げられない。これは何？

from itertools import product


def get_LCS(S1: str, S2: str):
    # dpテーブルの用意
    dp = [[0]*(len(S1)+1) for _ in range(len(S2)+1)]  # (#S2+1,#S1+1)の配列を作る
    # 境界条件
    # dp[0,:]とdp[:,0]は0。これは初期化の時点ですでにできている
    # dpの更新 Z字を書くように更新していく
    for i, j in product(range(len(S2)), range(len(S1))):
        # iが行、jが列
        if S2[i] == S1[j]:
            dp[i+1][j+1] = dp[i][j]+1
        else:
            dp[i+1][j+1] = max(dp[i][j+1], dp[i+1][j])  # 左と右の大きい方から取る
    return dp[-1][-1]


N = int(input())
for _ in range(N):
    S1 = input()
    S2 = input()
    print(get_LCS(S1, S2))
