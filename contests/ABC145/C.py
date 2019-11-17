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


import numpy as np
from itertools import permutations
from math import factorial
N = read_a_int()
zahyou = []
for _ in range(N):
    zahyou.append(read_ints())

zahyou = np.array(zahyou)
toori = factorial(N)
# print(toori)


def get_length(zahyou, keiro: list):
    zahyou1 = zahyou[keiro[:-1], :]
    zahyou2 = zahyou[keiro[1:], :]
    ret = np.linalg.norm(zahyou1 - zahyou2, axis=1).sum()
    return ret


ans = 0
for keiro in permutations(list(range(N))):
    ans += get_length(zahyou, keiro) / toori

print(ans)
