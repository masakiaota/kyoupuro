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


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


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


A = read_matrix(3)

import numpy as np
A = np.array(A)
idx_dict = {}
for i in range(3):
    for j in range(3):
        idx_dict[A[i, j]] = (i, j)
N = read_a_int()
B = read_col(N, 1)[0]

for b in B:
    if b not in idx_dict.keys():
        continue
    i, j = idx_dict[b]
    A[i, j] = -1

# print(A)
cnt = 0
for i in range(3):
    if (A[:, i] == -1).sum() == 3:
        print('Yes')
        exit()
    if (A[i, :] == -1).sum() == 3:
        print('Yes')
        exit()
    cnt += A[i, i] == -1

if cnt == 3:
    print('Yes')
    exit()

if A[0, 1] == -1 and A[1, 1] == -1 and A[2, 0] == -1:
    print('Yes')
    exit()

print('No')
