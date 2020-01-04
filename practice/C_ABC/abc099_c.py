# https://atcoder.jp/contests/abc099/tasks/abc099_c
# 典型的なコイン問題

# 引き出す金額の候補はNによって動的に決定する。
# dp[i] を i 円引き出すのに最小の操作回数とする
# dp[i] = min_{j \in 引き出し額}(dp[i - j]) +1 なので更新できる

# 今回は実装の都合上集めるdpではなく配るdpで実装する


def full(shape, full_value):
    if isinstance(shape, tuple):
        sha = shape[::-1]
        ret = [full_value] * sha[0]
        for s in sha[1:]:
            ret = [ret.copy() for i in range(s)]
        return ret

    if len(shape) == 2:
        return [[full_value] * shape[1] for _ in range(shape[0])]
    else:
        return [full_value] * shape


INF = 10**9

N = int(input())
candi = [1]
i = 1
while 6 ** i <= N:
    candi.append(6**i)
    i += 1

i = 1
while 9 ** i <= N:
    candi.append(9**i)
    i += 1

candi.sort()

dp = full(N + 1, INF)
dp[0] = 0  # 初期化

for i in range(N):
    for j in candi:
        if i + j > N:
            continue
        dp[i + j] = min(dp[i + j], dp[i] + 1)

print(dp[-1])
