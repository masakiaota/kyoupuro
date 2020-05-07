# 単純な重複組合せでないことに注意！
# 区別できる商品と区別できない商品が存在する。(高校まででやる重複組合せはすべての品物の個数が無限じゃないとき)

n = 3
m = 3
a = [1, 2, 3]

r'''
dp[i][j] ... [0,i)番目の品物からj個選ぶときの組み合わせの総数
更新則
dp[i+1][j] = sum_{k \in [max(0,j-a[i]),j+1) } dp[i][k]
ただし尺取的に考えれば
dp[i+1][j] = dp[i+1][j-1] + dp[i][j] - dp[i][j-a[i]-1](ただし添字が正のときのみ)
初期条件
dp[0][0] = 1
'''

dp = [[0] * (m + 1) for _ in range(n + 1)]
dp[0][0] = 1
for i in range(n):
    for j in range(m + 1):
        dp[i + 1][j] = dp[i + 1][j - 1] + \
            dp[i][j] - \
            (dp[i][j - a[i] - 1] if j - a[i] - 1 >= 0 else 0)

print(dp[n][m])
print(*dp, sep='\n')
