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
    tmp = ['#'*(W+2)]
    tmp = tmp + ['#' + read()[:-1] + '#' for _ in range(H)]
    tmp = tmp + ['#' * (W + 2)]
    return tmp


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


# 開始地点
start = []
end = []
for i, a in enumerate(A):
    for j, aa in enumerate(a):
        if aa == '#':
            end.append((i, j))
            if i == 0 or i == H + 1 or j == 0 or j == W + 1:
                continue
            start.append((i, j))


def mahattan(sy, sx, ey, ex):
    return abs(sy - ey) + abs(sx - ex)


ans = 0
# for i, j in product(range(H), range(W)):
#     if A[i][j] == '.':
#         tmp = 10000
#         for ey, ex in start:
#             tmp = min(tmp, mahattan(i, j, ey, ex))
#         ans = max(ans, tmp)

for i, j in start:
    tmp1 = 10000
    tmp2 = 10000
    tmp3 = 10000
    tmp4 = 10000

    for ey, ex in end:
        if ey >= i and ex > j:
            tmp1 = min(tmp1, mahattan(i, j, ey, ex))
        elif ey > i and ex <= j:
            tmp2 = min(tmp2, mahattan(i, j, ey, ex))
        elif ey <= i and ex < j:
            tmp3 = min(tmp3, mahattan(i, j, ey, ex))
        else:
            tmp4 = min(tmp4, mahattan(i, j, ey, ex))
        # print(tmp1, tmp2, tmp3, tmp4)
    tmp = max(tmp1, tmp2, tmp3, tmp4)
    ans = max(ans, tmp)

print(ans//2+1)
