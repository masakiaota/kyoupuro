# https://atcoder.jp/contests/abc095/tasks/arc096_b
# 写真参考


import sys
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations, accumulate
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


N, C = read_ints()
X, V = read_col(N)


def solve_max_a_plas_b(A: list, B: list):
    '''max_{i<j}(a_i + b_j)をO(n)で解く (古いpypyだとfuncが未実装かも)'''
    assert len(A) == len(B)
    A_accum = list(accumulate(A, func=max))
    B_accum = list(accumulate(reversed(B), func=max))[::-1]
    ret = 0
    for i in range(len(A_accum) - 1):
        ret = max(A_accum[i] + B_accum[i + 1], ret)
    return ret


def ret_candi(X, V):
    V_accum = list(accumulate(V))
    V_accum_r = list(accumulate(reversed(V)))[::-1]
    A, B = [], []
    katamiti_max = 0  # 必ず折れ曲がるとも限らないので
    for i in range(N):
        katamiti_max = max(katamiti_max, V_accum[i] - X[i])
        A.append(V_accum[i] - 2 * X[i])
        B.append((V_accum_r[i]) - (C - X[i]))
    # print(A, B)
    return A, B, katamiti_max


def solve(X, V):
    A, B, katamiti_max = ret_candi(X, V)
    return max(solve_max_a_plas_b(A, B), katamiti_max)


V_r = V[::-1]
X_r = []
for x in reversed(X):
    X_r.append(C - x)
print(max(solve(X, V), solve(X_r, V_r)))
