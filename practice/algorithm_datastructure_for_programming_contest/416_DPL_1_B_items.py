# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/1/DPL_1_B
# P418の下の方には、どんなitemを選んだか記録する方法も書いてある。これを実装する。


# 実装の都合上本とは少しだけ変えて実装する
# items (N,) ,i番目の品物の価値と重さが記録されているlist item[i]
# dp (N+1,W+1), [0,i)までのitemを考慮して大きさwのナップサックに入れる場合の価値の合計の最大値をdp[i][w]とする二次元list

# load data
N, W = list(map(int, input().split()))
items = []
for _ in range(N):
    items.append(tuple(map(int, input().split())))


# dpテーブルの作成
dp = [[-1] * (W + 1) for _ in range(N + 1)]

# dpテーブルの初期化
for i in range(N + 1):
    dp[i][0] = 0
for w in range(W + 1):
    dp[0][w] = 0

# dpテーブルの更新
from itertools import product
for i, w in product(range(N), range(W + 1)):
    dp[i + 1][w] = dp[i][w]
    value, weight = items[i]
    if w - weight >= 0:
        dp[i + 1][w] = max(dp[i][w], dp[i][w - weight] + value)

print(dp[-1][-1])
