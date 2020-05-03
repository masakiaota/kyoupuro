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
from itertools import product, permutations, combinations, combinations_with_replacement
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd

# dを大きい方から取って行きたい
# A1は1として問題ない。
# 数列を仮定して探索すれば良い
N, M, Q = read_ints()
DATA = read_tuple(Q)
A = [1 + x for x in range(M)]


def ret_score(A):
    ret = 0
    for a, b, c, d in DATA:
        if A[b - 1] - A[a - 1] == c:
            ret += d
    return ret


ans = 0
for AA in combinations_with_replacement(A, r=N):
    ans = max(ans, ret_score(AA))
print(ans)
