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
from math import gcd


# i<jに対して
# Ai-i = -Aj-j か Ai+i=-Aj+jがなり立つ組を探せばよい
N = read_a_int()
A = read_ints()


def ret_kumiawase(X, Y):  # どっちもリスト
    ret = 0
    cnt = Counter()
    for i in range(N - 1, -1, -1):
        ret += cnt[X[i]]  # X[i]の数字の個数を加算する
        cnt.update([Y[i]])  # X[i]の数字の個数を加算する
    return ret


# 前者から
Ai = []
for i, a in enu(A):
    Ai.append(a - i)
Aj = []
for i, a in enu(A):
    Aj.append(-a - i)
ans = ret_kumiawase(Ai, Aj)


# 後者
Ai = []
for i, a in enu(A):
    Ai.append(a + i)
Aj = []
for i, a in enu(A):
    Aj.append(-a + i)
ans += ret_kumiawase(Ai, Aj)
print(ans)
