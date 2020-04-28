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

# https://atcoder.jp/contests/abc125/tasks/abc125_d

# 理想としてはすべて+にしたい
# i<=N-1ならばすべて自由にflipできるのでは？ #まえから愚直にflipのシミュレーションをしていっても良いしな
# iを都合のいいように変えたときには、i+1も操作で都合のいいように変えられる。よって後ろに押し付けられる
# つまり後ろの2要素の大小関係のみが重要
# んー前から愚直にシミュレーション
# すべて正にできないときは1個だけ負になるのだから、absの最小を持っておけば良い

N = read_a_int()
A = read_ints()
ans = 0
for i in range(N - 1):
    if A[i] < 0:
        ans += -A[i]
        A[i + 1] = -A[i + 1]
    else:
        ans += A[i]
if A[N - 1] >= 0:
    ans += A[N - 1]
else:
    ans += -A[N - 1]
    min_abs = abs(min(A, key=abs))
    # print(ans, min_abs)
    ans -= min_abs * 2
print(ans)
