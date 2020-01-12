# 誤読したか？

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


N, M = read_ints()
from collections import defaultdict
flgs = [False] * N  # N問をACしたか
n_wa_ls = [0] * N


for _ in range(M):
    p, s = input().split()
    p = int(p) - 1
    if flgs[p]:
        continue
    if s == 'AC':
        flgs[p] = True
    elif s == 'WA':
        n_wa_ls[p] += 1

n_wa = 0
for i in range(N):
    if flgs[i]:
        n_wa += n_wa_ls[i]

print(sum(flgs), n_wa)
