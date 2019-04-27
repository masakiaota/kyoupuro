# https://qiita.com/drken/items/a5e6fe22863b7992efdb#問題-3部分和問題

# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read() for _ in range(H)]


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


N = read_a_int()
a = read_ints()
A = read_a_int()

import numpy as np

dp = np.full((N + 1, A + 1), False, dtype='bool')

dp[0, 0] = True

from itertools import product
for i, j in product(range(N), range(A + 1)):
    if j - a[i] < 0:
        dp[i + 1, j] = dp[i, j]
    else:
        dp[i + 1, j] = dp[i, j] or dp[i, j - a[i]]

if dp[-1, -1]:
    print('YES')
else:
    print('NO')
# print(dp[-1, -1])
