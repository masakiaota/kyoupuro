# https://atcoder.jp/contests/arc043/tasks/arc043_a
# 平均A、幅Bとなるように線形変換を施すだけ
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce

N, A, B = ints()
S, = read_col(N)
B_old = max(S) - min(S)
if B_old == 0:
    exit(-1)

P = B / B_old
A_old = sum(P * s for s in S) / len(S)

print(B / B_old, A - A_old)
