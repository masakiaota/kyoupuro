# https://atcoder.jp/contests/abc129/tasks/abc129_c
# 典型的なDP


from copy import deepcopy


def full(shape, full_value):
    if isinstance(shape, tuple):
        if len(shape) == 2:
            return [[full_value] * shape[1] for _ in range(shape[0])]
        else:
            import numpy as np
            return np.full(shape, full_value).tolist()
    else:
        return [full_value] * shape


MOD = 10**9 + 7
# dp[i]をi段目までの通りの数とする。
N, M = list(map(int, input().split()))
to_zero = full((N + 1), False)
for _ in range(M):
    to_zero[int(input())] = True  # 床が抜けているところ

# dpテーブルの用意
dp = full((N + 1), -1)
dp[0] = 1
dp[1] = 0 if to_zero[1] else 1
for i in range(2, N + 1):
    dp[i] = 0 if to_zero[i] else dp[i - 2] + dp[i - 1]
    if dp[i] > MOD:
        dp[i] %= MOD

print(dp[-1])
