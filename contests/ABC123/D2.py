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


X, Y, Z, K = read_ints()
A = read_ints()
B = read_ints()
C = read_ints()
A.sort(reverse=True)
B.sort(reverse=True)
C.sort(reverse=True)


import numpy as np
dp = np.zeros((X, Y, Z), dtype=int)
dp[0, 0, 0] = A[0] + B[0] + C[0]

from itertools import product
for a, b, c in product(range(X), range(Y), range(Z)):
    if a + 1 != X:
        dp[a + 1, b, c] = dp[a, b, c] - A[a] + A[a + 1]
    if b + 1 != Y:
        dp[a, b + 1, c] = dp[a, b, c] - B[b] + B[b + 1]
    if c + 1 != Z:
        dp[a, b, c + 1] = dp[a, b, c] - C[c] + C[c + 1]

# K = min(K, X*Y*Z)
ls = sorted(dp.reshape(-1), reverse=True)
for ans in ls[:K]:
    print(ans)
