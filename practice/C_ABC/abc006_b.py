# https://atcoder.jp/contests/abc006/tasks/abc006_2
# 再帰メモ化で行けそう →なぞのエラーでできなかったのでおとなしくdpで


MOD = 10007


def tribonacci(a):
    if a == 1 or a == 2:
        return 0
    elif a == 3:
        return 1
    dp = [None] * a
    dp[0] = 0
    dp[1] = 0
    dp[2] = 1
    for aa in range(3, a):
        dp[aa] = (dp[aa - 1] + dp[aa - 2] + dp[aa - 3]) % MOD

    return dp[-1]


n = int(input())
print(tribonacci(n))
