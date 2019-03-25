# DPを用いて解く。
# 少し複雑なので、自分の解説と照らし合わせて読むこと。
MOD = 10**9+7
N = int(input())

# AGCT→0123として扱う

# dpテーブルの作成shape=(101, 3,3,3) #長さが0~100まで扱いたいため
import numpy as np
dp = np.zeros((N+1, 4, 4, 4), dtype="int64")

dp[0, 3, 3, 3] = 1  # 解説放送流テクニカルな楽な初期化
# 末尾がc3,c2,c1で終わる長さlの文字列の通りの数を計算していく
# print(dp)
for l in range(N):
    for c3 in range(4):
        for c2 in range(4):
            for c1 in range(4):
                # すでに条件を満たしていないのでスキップ(なくても多分平気)
                if dp[l, c3, c2, c1] == 0:
                    continue
                # 最後につける一文字を考える
                for a in range(4):
                    # あかん条件
                    if a == 1 and c1 == 2 and c2 == 0:
                        continue
                    if a == 2 and c1 == 1 and c2 == 0:
                        continue
                    if a == 2 and c1 == 0 and c2 == 1:
                        continue
                    if a == 2 and c1 == 1 and c3 == 0:
                        continue
                    if a == 2 and c2 == 1 and c3 == 0:
                        continue
                    dp[l + 1, c2, c1, a] += dp[l, c3, c2, c1] % MOD
                    dp[l + 1, c2, c1, a] %= MOD

print(dp[N, :, :, :].sum() % MOD)
