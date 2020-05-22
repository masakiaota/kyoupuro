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


# https://atcoder.jp/contests/abc135/tasks/abc135_c
# 前から貪欲に倒していくのが最適
# ある勇者iについて, 街iをできる限り倒す。
# 余裕があったら街i+1を減らしておく

N = read_a_int()
A = read_ints()
B = read_ints()

ans = 0
for i in range(N):
    atk = B[i]
    if atk < A[i]:  # 倒しきれないとき
        ans += atk
        A[i] -= atk
    else:  # 倒し切れるとき
        ans += A[i]
        atk -= A[i]
        A[i] = 0
        ans += min(atk, A[i + 1])
        A[i + 1] = max(A[i + 1] - atk, 0)
print(ans)
