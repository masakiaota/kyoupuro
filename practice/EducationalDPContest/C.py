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

# 全探索だと
# 3^N通りになってTLEだこれ

# 制限がなければ、iごとにmaxを取っておわりO(N)

# iの時の最善の状態が[i,j](jは行動)のとき、i+1最善の状態は[i+1,k](k!=j)となる
# これはDPで実装できそう！


# import numpy as np

# N = read_a_int()
# ABC = np.array(read_matrix(N))
# # A, B, C = read_col(N, 3)

# dp = np.full((N, 3), 0, dtype='int16')
# # 0->a,1->b,2->c

# dp[0, :] = ABC[0]

# for i in range(N - 1):
#     for j, abc in enumerate(ABC[i+1]):
#         dp[i + 1, j] = dp[i, [k for k in range(3) if k != j]].max() + abc

# # print(dp)
# print(dp[-1].max())

# TLEしたのでpypy3仕様にかきなおし

N = read_a_int()
ABC = read_matrix(N)

# dp = np.full((N, 3), -float('inf'))
dp = [[-float('inf') for _ in range(3)] for _ in range(N)]
# 0->a,1->b,2->c

for i in range(3):
    dp[0][i] = ABC[0][i]

for i in range(N - 1):

    for j in range(3):
        dp[i+1][j] = max([dp[i][k] + ABC[i+1][j] for k in range(3) if k != j])

print(int(max(dp[-1])))
