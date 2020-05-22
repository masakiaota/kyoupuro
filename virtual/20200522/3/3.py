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

# https://atcoder.jp/contests/abc065/tasks/arc076_a

# 0のとき abs(N-M)>1 必ず隣り合うところができてしまう
# それ以外のとき、Nの順列×Mの順列×犬と猿の並べ方
# 並べ方→同数→×2
# 異なる場合→×1

N, M = read_ints()
if abs(N - M) > 1:
    print(0)
    exit()

N_per = 1
M_per = 1
for i in range(1, N + 1):
    N_per *= i
    N_per %= MOD

for i in range(1, M + 1):
    M_per *= i
    M_per %= MOD


print((2 if N == M else 1) * N_per * M_per % MOD)
