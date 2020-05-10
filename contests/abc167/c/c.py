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

N, M, X = read_ints()
C, *A = read_col(N)


def ret_each_exp(p):
    exps = [0] * M
    money = 0
    # print(p)
    for i, pp in enu(p):
        if pp == 0:
            continue
        for m in range(M):
            exps[m] += A[m][i]
        money += C[i]
    return exps, money


def is_valid(exps):
    for e in exps:
        if e < X:
            return False
    return True


ans = INF
for p in product(range(2), repeat=N):
    exps, money = ret_each_exp(p)
    if is_valid(exps):
        ans = min(ans, money)
print(ans if ans != INF else -1)
