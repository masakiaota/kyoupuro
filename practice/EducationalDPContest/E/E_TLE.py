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


N, K = read_ints()
W, V = read_col(N, 2)

import numpy as np
# 流れ
# dpテーブルのサイズを決める
#     (N,max(V)*N)
# dpテーブルの初期化
# dpテーブルの更新
#     いままでと同じ
#     sum_v以上を達成すhるように選んだときのsum_wの最小値を保持する
# 答えの抽出
# dpでW以下かつ最大価値が最大(sum_v)なものが答え


dp = np.full((N+1, np.max(V) * N+1), float('inf'))
# dp[:, 0] = 0
dp[0][0] = 0

from itertools import product

for i, sum_v in product(range(N), range(np.max(V) * N+1)):
    if sum_v - V[i] < 0:
        dp[i + 1, sum_v] = dp[i, sum_v]
    else:
        # print(i, sum_v)
        dp[i + 1, sum_v] = min(
            dp[i, sum_v],
            dp[i, sum_v-V[i]]+W[i]
        )

# print(dp)
print(np.where((dp <= K).sum(axis=0) != 0)[0].max())
# print(max(np.where((dp < K).sum(axis=0) != 0)))
# print(max(np.where((dp < K).sum(axis=0) != 0)))
