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


def combination_mod(n, r, mod):
    # if r > n:
    #     return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


def ret_list_combination_mod(n, r, mod):
    ret = [1]
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    # r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
        ret.append(nf * pow(rf, mod - 2, mod) % mod)
    return ret

# https://atcoder.jp/contests/abc156/tasks/abc156_e
# 時間内に通せなかったのは反省
# n-k個以上の部屋が1人以上の通りの数がほしい→少なくともn-k個の部屋が埋まっている→0人がa個ある時の通りの数 sum a=0..k


MOD = 10**9 + 7
n, k = read_ints()
# n<=kのときは簡単
if n <= k:
    print(combination_mod(n + n - 1, n, MOD))
    exit()
# n>kのときはどうしよう

ans = 1
nCa = ret_list_combination_mod(n, k, MOD)
n_1Ca = ret_list_combination_mod(n - 1, k, MOD)
for a in range(1, k + 1):
    ans += nCa[a] * n_1Ca[a]  # ここ高速化しないと厳しいかな
    ans %= MOD
print(ans)
