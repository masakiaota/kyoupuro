# https://atcoder.jp/contests/abc029/tasks/abc029_d
# Nまでの1を何回書くか
# 0001 0002 0003 ... 9999
# dp[i][j][1の登場回数] ... を満たす数字の数 というdpテーブルを作成する

N = '0' + input()

dp = [[[0] * 10 for _ in range(2)] for _ in range(len(N))]
dp[0][0][0] = 1
for i in range(len(N) - 1):
    for j in range(2):
        for d in range(10 if j else int(N[i + 1]) + 1):
            # d==1のときだけ1のカウントが増える
            if d == 1:
                for k in range(9):
                    dp[i + 1][j or d < int(N[i + 1])][k + 1] += dp[i][j][k]
            else:
                for k in range(10):
                    dp[i + 1][j or d < int(N[i + 1])][k] += dp[i][j][k]

# print(dp)
# 1の登場回数をカウントする
ans = 0
for i in range(10):
    ans += (dp[-1][1][i] + dp[-1][0][i]) * i

print(ans)
