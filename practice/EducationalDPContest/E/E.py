import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


import numpy as np
from numba import njit

N, W_max = ints()
W, V = read_col(N)
W = np.array(W, dtype=np.int64)
V = np.array(V, dtype=np.int64)

'''
dp[i,j] ... 重さの最小 (:iまで考慮したとき、価値がちょうどjのとき)
答えはW以下の数字が入ってるマスの一番左側

更新則
chmin(dp[i+1,j+V[i]] , dp[i, j]+W[i]) #ナップサックに入れた場合
chmin(dp[i+1,j] , dp[i, j]) # ナップサックに入れなかった場合
'''


@njit('(i8, i8, i8[:], i8[:])', cache=True)
def solve(N, W_max, W, V):
    V_max = np.sum(V) + 1
    dp = np.full((N + 1, V_max), 10**12, dtype=np.int64)
    # 初期化
    dp[0][0] = 0  # 一個も選ばず価値が0なら必ず重さも0

    # 更新
    for i in range(N):
        for j in range(V_max):
            jv = j + V[i]
            if jv < V_max:
                dp[i + 1, jv] = min(dp[i + 1, jv], dp[i, j] + W[i])
            dp[i + 1, j] = min(dp[i + 1, j], dp[i, j])

    # 左から見てく
    for j in range(V_max - 1, -1, -1):
        if np.any(dp[:, j] <= W_max):
            print(j)
            return


solve(N, W_max, W, V)
