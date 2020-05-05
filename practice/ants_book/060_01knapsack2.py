n = 4
W = [2, 1, 3, 2]
V = [3, 2, 4, 2]
W_max = 5

'''
dp[i][j] ... 達成可能な重さの総和の最小。[0,i)で価値がjになるように選んだとき。
更新則
dp[i+1][j] = min(dp[i][j], dp[i][j-V[i]]+W[i])
初期値
dp[i][j]=INF (for any i,j)
dp[0][0]=0
'''
INF = float('inf')
V_sum = sum(V)

dp = [[INF] * (V_sum + 1) for _ in range(n + 1)]
dp[0][0] = 0

for i in range(n):
    for j in range(V_sum + 1):
        dp[i + 1][j] = min(dp[i + 1][j], dp[i][j])
        if j - V[i] < 0:
            continue
        dp[i + 1][j] = min(dp[i + 1][j], dp[i][j - V[i]] + W[i])

# dp[n][j]<=Wを満たす最大のj
for j in range(V_sum, -1, -1):
    if dp[n][j] <= W_max:
        print(j)
        break

# from pprint import pprint
# pprint(dp)
