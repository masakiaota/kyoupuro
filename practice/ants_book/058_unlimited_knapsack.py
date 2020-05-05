n = 3
W = [3, 4, 2]
V = [4, 5, 3]
W_max = 7

'''
dp[i][j] ... 価値の総和の最大。[0,i)の中から選んで、重さがj以下であるときの。
更新
dp[i+1][j] = max(dp[i][j], dp[i+1][j-W[i]]+V[i])
'''
# こっちのほうがコイン問題の更新を行うよりもシンプル！(本質は同じ)
dp = [[0] * (W_max + 1) for _ in range(n + 1)]
for i in range(n):
    for j in range(W_max + 1):
        dp[i + 1][j] = max(dp[i + 1][j], dp[i][j])
        if j - W[i] < 0:
            continue
        dp[i + 1][j] = max(dp[i + 1][j], dp[i + 1][j - W[i]] + V[i])

print(dp[-1][-1])
