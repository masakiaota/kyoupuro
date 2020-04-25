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

# https://atcoder.jp/contests/abc107/tasks/arc101_a
N, K = read_ints()
X = read_ints()

# 右だけにK本とるとき
# 右から左に折り返してK本とるとき
# 反転させて二通りやるときの場合分けをする


def solve(X):
    ret = 10**13
    for i in range(N):
        j = i + K - 1
        if j >= N:
            continue
        if X[i] <= 0 <= X[j]:
            ret = min(ret, -2 * X[i] + X[j])  # 右スタートか左スタートかの違い
            ret = min(ret, 2 * X[j] - X[i])
    return ret


ans = solve(X)
# 右直進だけのほうがいいとき
i = bisect_left(X, 0) + K - 1
if i < N:
    ans = min(ans, X[i])
# 左直進だけのほうがいいとき
i = bisect_right(X, 0) - K
if -1 < i:
    ans = min(ans, -X[i])
# print(i, X[i])
print(ans)
