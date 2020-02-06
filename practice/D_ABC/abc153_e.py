# https://atcoder.jp/contests/abc153/tasks/abc153_e
# コイン問題の応用で解けそう

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


H, N = read_ints()
A, B = read_col(N, 2)
minb = min(B)
maxa = max(A)
INF = 10**10 + 1

dp = [INF] * (H + 1 + maxa)
dp[0] = 0

for i in range(1, H + 1 + maxa):
    for a, b in zip(A, B):
        if i - a < 0:
            continue
        dp[i] = min(dp[i], dp[i - a] + b)

# if dp[H] < INF:
#     print(dp[H])
#     exit()
# これが悪かったけどなんで悪いのかはわからない...
# いや、わかったわ

print(min(dp[H:]))
