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


def read_map(H, W):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


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


H, W = read_ints()
A = read_map(H, W)


# print(A)

from itertools import product
import numpy as np

# 黒の座標
ey, ex = [], []
# 白の座標
start = []
for i, a in enumerate(A):
    for j, aa in enumerate(a):
        if aa == '#':
            ey.append(i)
            ex.append(j)
        else:
            start.append((i, j))


ey = np.array(ey)
ex = np.array(ex)


def mahattan(sy, sx, ey, ex):
    return np.abs(sy - ey) + np.abs(sx - ex)


ans = 0

for i, j in start:
    tmp = mahattan(i, j, ey, ex).min()
    ans = max(ans, tmp)

print(ans)
