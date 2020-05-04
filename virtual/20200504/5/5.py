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

# https://atcoder.jp/contests/abc128/tasks/abc128_d
N, K = read_ints()
V = read_ints()

ans = 0
for i, j in combinations_with_replacement(range(N + 1), r=2):
    # V[0:i]とV[j:]はとったことにしてみる
    k = i + (N - j)
    if k > K:
        continue
    Vsub = V[:i] + V[j:]
    Vsub.sort(reverse=True)
    while Vsub and k < K and Vsub[-1] < 0:
        k += 1
        Vsub.pop()
    ans = max(sum(Vsub), ans)
print(ans)
