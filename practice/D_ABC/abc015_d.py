# https://atcoder.jp/contests/abc015/tasks/abc015_4
# ナップサック問題 k枚以下で最大の価値の状態も入れてあげる

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


from itertools import product
from array import array
W = read_a_int()
N, K = read_ints()
AB = read_tuple(N)

# 枚数も伝播するdp
# dp[i][j][k]...i-1番目までの商品まで考慮したとき、重さj以下、枚数k以下で達成できる価値の最大値
dp = [[[0] * (K + 1) for _ in range(W + 1)] for _ in range(2)]  # 初期化も同時に
for i, j, k in product(range(N), range(1, W + 1), range(1, K + 1)):
    w, v = AB[i]
    dp[(i + 1) & 1][j][k] = max(
        dp[i & 1][j - w][k - 1] + v if j - w >= 0 else 0,
        dp[i & 1][j][k])


# from pprint import pprint
# pprint(dp)
print(dp[(i + 1) & 1][-1][-1])
