import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


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


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc095/tasks/arc096_b
N, C = read_ints()
X, V = read_col(N)
# 遠くの寿司も見ないとダメそう(vがバカでかい可能性がある)
# とりあえず右スタートを前提に考える。歩く距離の最小は2パターン。
# 1. 右に進んで食べるだけ
# 2. 途中まで右に進んで途中で左に戻る


def ret_ans1(X, V):  # 右スタートを想定
    sum_v = 0
    ans = -1
    for x, v in zip(X, V):
        sum_v += v
        ans = max(ans, sum_v - x)
    return ans


def ret_ans2(X, V):  # 右スタートを想定
    Z = []
    sum_v = 0
    for x, v in zip(X, V):
        sum_v += v
        Z.append(sum_v - 2 * x)

    Y = []
    sum_v = 0
    for x, v in reversed(list(zip(X, V))):
        sum_v += v
        Y.append(sum_v - (C - x))
    Y = list(reversed(Y))

    # optimize
    ans = max_sum(Z, Y)
    return ans


def max_sum(X, Y):
    # X_i + Y_j (i<j)を最大化する O(n)
    X_accum = [-10**15]
    for x in X:
        X_accum.append(max(x, X_accum[-1]))
    del X_accum[0]
    Y_accum = [-10**15]
    for y in reversed(Y):
        Y_accum.append(max(y, Y_accum[-1]))
    Y_accum = list(reversed(Y_accum))
    del Y_accum[-1]
    ret = 0
    for i in range(len(X) - 1):
        ret = max(ret, X_accum[i] + Y_accum[i + 1])
    return ret


ans = 0
ans = max(ans, ret_ans1(X, V))
ans = max(ans, ret_ans2(X, V))

# 左周りにスタートする
V = list(reversed(V))
X_r = []
for x in reversed(X):
    X_r.append(C - x)

ans = max(ans, ret_ans1(X_r, V))
ans = max(ans, ret_ans2(X_r, V))
print(ans)
