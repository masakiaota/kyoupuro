n = 3
a = [3, 5, 8]
m = [3, 2, 2]
K = 17

'''
dp[i][j] ... ちょうどjをつくるときの、a[i]の余りの最大個数。(作れないときは-1)
更新則
dp[i+1][j] = m[i] for (dp[i][j]>=0)
dp[i+1][j] = dp[i+1][j-a[i]] - 1 for (上記でない かつ dp[i+1][j-a[i]] > 0)
dp[i+1][j] = -1 for (dp[i][j]<0 and dp[i][j-a[i]] < 0)
初期条件
dp[0][0] = 0 #これで十分駆動するはず
'''

dp = [[-1] * (K + 1) for _ in range(n + 1)]
dp[0][0] = 0
for i in range(n):
    for j in range(K + 1):
        if dp[i][j] >= 0:
            dp[i + 1][j] = m[i]
        else:
            if j - a[i] < 0:
                continue
            if dp[i + 1][j - a[i]] > 0:
                dp[i + 1][j] = dp[i + 1][j - a[i]] - 1

print('Yes' if dp[n][K] >= 0 else 'No')
