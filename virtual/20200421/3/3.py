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
# https://atcoder.jp/contests/abc143/tasks/abc143_d
N = read_a_int()
L = read_ints()
L.sort()
ans = 0
for i in range(N):
    for j in range(i + 1, N):
        a = L[i]
        b = L[j]
        bigest_idx = bisect_left(L, a + b, lo=j + 1)
        smallest_idx = bisect_right(L, b - a, lo=j + 1)
        tmp = bigest_idx - smallest_idx
        # if smallest_idx <= i < bigest_idx:
        #     tmp -= 1
        # if smallest_idx <= j < bigest_idx:
        #     tmp -= 1
        ans += tmp
        # print(i, j, smallest_idx, bigest_idx, tmp)
print(ans)
