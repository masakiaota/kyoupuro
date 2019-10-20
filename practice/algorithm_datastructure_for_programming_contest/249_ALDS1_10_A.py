# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/10/ALDS1_10_A
# やるだけ


n = int(input())
# dpテーブルの用意
dp = [-1]*(n+1)
# dpの初期値
dp[0], dp[1] = 1, 1

for i in range(2, n+1):
    dp[i] = dp[i-1]+dp[i-2]
print(dp[-1])
