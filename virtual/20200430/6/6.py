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


def combination_mod(n, r):
    # mod取りながらcombination
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i)
        rf = rf * (i + 1)
    return nf // rf


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc057/tasks/abc057_d
# ソートしておけば、平均の最大化は簡単 (上位A個選べばよい)
# 通りの数は？
# もし単調増加なら必ず1通り

# 平均の最大化が変わらないとは...?
# 大きい方からA個目の数(cとする)が複数ある場合は別のを選らべる(通りが増える)
# 具体的にはn_c C rの通りの数が存在する(rは上位A個の中に存在するcの数)

# 三種類の場合があるな
# 単調増加のときは簡単

N, A, B = read_ints()
V = read_ints()
V.sort(reverse=True)
V_A = V[:A]
ans1 = sum(V_A) / A
# 単調増加の場合
if V[A] != V[A - 1]:
    print(ans1)
    print(1)
    exit()

# A個選ぶのが最適の場合
if V_A[0] != V_A[-1]:
    c = V_A[-1]
    n_c = V.count(c)
    r = V_A.count(c)
    # print(n_c, r)
    ans2 = combination_mod(n_c, r)
    print(ans1)
    print(ans2)
    exit()

# 全部同じ場合
n_same = V.count(V[0])
if n_same > A:
    ans2 = 0
    r_max = min(n_same, B)  # 最大これだけ選べる
    for i in ra(A, r_max + 1):
        ans2 += combination_mod(n_same, i)
    print(ans1)
    print(ans2)
    exit()


raise ValueError('CE出てほしい')
