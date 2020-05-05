# 再帰関数版は省略、P55にあるDPを行う

N = 4
W = [2, 1, 3, 2]
V = [3, 2, 4, 2]
W_max = 5

'''
dp[i][j] ... 達成可能な最大の価値。[0,i)までの品物を重さがj以下で選んだとき。
更新式
dp[i+1][j] = max(dp[i][j], dp[i][j-W[i]] + V[i]) #i番目を取るか取らないかで良い方を選択する。
初期条件
dp[0][:]=0 品物を選んでないなら価値はない
dp[:][0]=0 重さ0では品物を選べない→価値はない
'''

dp = [[0] * (W_max + 1) for _ in range(N + 1)]

for i in range(N):
    for j in range(W_max + 1):
        dp[i + 1][j] = max(dp[i + 1][j], dp[i][j])
        if j - W[i] >= 0:
            dp[i + 1][j] = max(dp[i + 1][j], dp[i][j - W[i]] + V[i])

print(dp[-1][-1])
