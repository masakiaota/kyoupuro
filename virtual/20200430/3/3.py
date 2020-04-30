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

# https://atcoder.jp/contests/arc091/tasks/arc091_a
# 角に存在するのは4回→表になる
# 端にあるのは6回→表になる
# 内側に存在するカードは9回フリップする→全部裏になる
# よって答えは基本的にN-2 * M-2 ただし(N,M>=2)
# コーナーケース N and M ==1のとき→1
# N == 1のとき M-2が答え


N, M = read_ints()
N, M = (N, M) if N < M else (M, N)

if N == 1 and M == 1:
    print(1)
    exit()
if N == 1:
    print(M - 2)
    exit()
print((N - 2) * (M - 2))
