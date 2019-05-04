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
    return [list(read()[:-1]) for _ in range(H)]


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
A = read_map(H)


def not_all_black(A):
    for a in A:
        for aa in a:
            if aa == '.':
                return False
    return True


from itertools import product
from copy import deepcopy

mvx = (1, 0, -1, 0)
mvy = (0, 1, 0, -1)

cost = 0
while True:
    if not_all_black(A):
        break

    A_tmp = deepcopy(A)
    # 一つ進める
    for i, j in product(range(H), range(W)):
        if A[i][j] == '.':
            for di, dj in zip(mvy, mvx):
                i_new, j_new = i + di, j + dj
                if not (- 1 < i_new < H) or not (- 1 < j_new < W):
                    continue

                if A[i_new][j_new] == '#':
                    A_tmp[i][j] = '#'
                    break
    A = deepcopy(A_tmp)
    cost += 1
    # print(A)

# print(A)
print(cost)
