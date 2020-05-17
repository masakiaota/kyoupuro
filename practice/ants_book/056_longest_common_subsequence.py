n = 4
m = 4
s = 'abcd'
t = 'becd'

'''
dp[i][j] ... s[:i]とt[:j]のLCS長
更新則
dp[i+1][j+1] = max(dp[i][j+1],dp[j+1][j], dp[i][j]+1) #3項目はs[i]==t[j]のときだけ
∵同じ文字だったら最長が1が増える。そうじゃなかったら最長の方を選ぶ
'''
dp = [[0] * (m + 1) for _ in range(n + 1)]

# # 初期化を明示的に書くと下のように成るけどすべての要素を0で初期化してるので今はいらない
# for i in range(n + 1):
#     dp[i][0] = 0
# for j in range(m + 1):
#     dp[0][j] = 0

for i in range(n):
    for j in range(m):
        dp[i + 1][j + 1] = max(dp[i][j + 1],
                               dp[i + 1][j],
                               (dp[i][j] + 1) if s[i] == t[j] else 0)

print(dp[-1][-1])
