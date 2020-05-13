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

# https://atcoder.jp/contests/abc068/tasks/arc079_b
# 重要な考察
# すべての要素が等しい数列に関しては,N回操作を行うと必ずすべての要素が-1される

# K//N回 操作したことにして
# のこりのK%N回は愚直に操作したことにする

K = read_a_int()
N = 50

kn = K // N
A = [N - 1 + kn] * N

rest = K % N
# 愚直に残りの回数シミュレーション
for i in range(rest):
    mi = min(A)
    idx = A.index(mi)
    for j in range(N):
        if idx == j:
            A[j] += N
        else:
            A[j] -= 1

print(N)
print(*A)
