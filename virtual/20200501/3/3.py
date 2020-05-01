
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

# https://atcoder.jp/contests/abc047/tasks/abc047_b
# a=1のなかで一番大きい値をXとするとき、x<Xはすべて黒く塗られる
# a=2のなかで一番小さい値Xとするとき、X<xはすべて黒く塗られる
# a=3→一番大きいY、y<Y
# a=4→一番小さいY、Y<y は塗る
W, H, N = read_ints()
A = [0, W, 0, H]
for _ in range(N):
    x, y, a = read_ints()
    a -= 1
    if a == 0:
        A[a] = max(x, A[a])
    if a == 1:
        A[a] = min(x, A[a])
    if a == 2:
        A[a] = max(y, A[a])
    if a == 3:
        A[a] = min(y, A[a])

if A[1] - A[0] > 0 and A[3] - A[2] > 0:
    print((A[1] - A[0]) * (A[3] - A[2]))
else:
    print(0)

#