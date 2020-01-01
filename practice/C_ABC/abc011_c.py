# https://atcoder.jp/contests/abc011/tasks/abc011_3
# 動的計画法で解いたがあとで考えたら貪欲法でもいけるやん...

INF = 10**9

N = int(input())
NGs = []
for _ in range(3):
    NGs.append(int(input()))

if N in NGs:
    print('NO')
    exit()

# dp[i] ... iを作るのに必要な処理の回数
dp = [INF] * (N + 3)  # dp[-1],dp[-2]はループ時にアクセスする用
dp[0] = 0
for i in range(1, N + 1):
    if i in NGs:
        continue
    dp[i] = min(dp[i - 3], dp[i - 2], dp[i - 1]) + 1

if dp[i] > 100:
    print('NO')
else:
    print('YES')
