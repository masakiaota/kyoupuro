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


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


from numba import njit
import numpy as np


N, K = ints()
W, V = read_col(N)
W = np.array(W, dtype=np.int64)
V = np.array(V, dtype=np.int64)


@njit('(i8,i8,i8[:],i8[:])', cache=True)
def main(N, K, W, V):
    dp = np.zeros((N + 1, K + 1), dtype=np.int64)
    for i in range(N + 1):
        dp[i, 0] = 0

    for i in range(N):
        w, v = W[i], V[i]
        for sum_w in range(K + 1):
            if sum_w - w < 0:
                dp[i + 1, sum_w] = dp[i, sum_w]
            else:
                dp[i + 1, sum_w] = max(
                    dp[i, sum_w],
                    dp[i, sum_w - w] + v)
    print(dp[-1, -1])


main(N, K, W, V)
