# https://atcoder.jp/contests/agc017/tasks/agc017_a
# dpっぽくね？

'''
dp[i][p] ... 袋の選び方の通りの総数 [0,i)まで見たとき、余りがpになる場合


if A[i]が偶数
選ぶときの通りの数
    dp[i+1][0] += dp[i][0]*2
    dp[i+1][1] += dp[i][1]*2
    ∵偶+偶→偶、奇+偶→奇 でもともとの通りの数に影響を与えない
if A[i]が奇数
    dp[i+1][0] += dp[i][1]+dp[i][0]
    dp[i+1][1] += dp[i][0]+dp[i][1]
    ∵偶+奇→奇、奇+奇→偶 で遷移するので選ぶときと選ばないときで遷移先が異なる
'''
N, P = map(int, input().split())
A = list(map(int, input().split()))
dp0 = 1
dp1 = 0

for a in A:
    if a & 1:
        new0 = dp0 + dp1
        new1 = new0
    else:
        new0 = dp0 * 2
        new1 = dp1 * 2
    dp0 = new0
    dp1 = new1

print(dp1 if P & 1 else dp0)
