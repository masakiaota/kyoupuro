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

# ダメなイワシのペアを高速で発見したい
# i について固定したときにどうにか良い性質が見つからないかな
# AiとBiのgcdで割ったときにa,bとすると、Aj=bの倍数、Bj=aの倍数であれば良い...？
# 倍数を高速に管理することは可能か？

# 適当に実装してみる
N = read_a_int()
A, B = read_col(N)
ABG = []
ABG2 = []
for a, b in zip(A, B):
    d = gcd(a, b)
    ABG.append((a // d, b // d))
    ABG2.append((-b // d, a // d))
cnt = Counter(ABG)
ans = pow(2, N, MOD) - 1
print(ans)
for a, b in ABG2:
    n = cnt[(a, b)]
    if n > 0:
        print(n)
        ans -= pow(2, n, MOD)
        ans %= MOD
print(ans)


print(cnt)
