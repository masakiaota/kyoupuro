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


MOD = 998244353
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


def combination_mod(n, r, mod=MOD):
    # mod取りながらcombination
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


def ret_list_comb_r(n, r, mod=MOD):
    '''nC[0:r+1]を返す。for とかで再計算せずに済むように'''
    ret = [1]
    if r > n:
        raise ValueError('rがnより大きいけど大丈夫か？(0通り？)')
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
        ret.append(nf * pow(rf, mod - 2, mod) % mod)
    return ret


N, M, K = read_ints()
N_1C = ret_list_comb_r(N - 1, K)


def ret_ans_when_k(k):
    ret = M
    ret *= pow(M - 1, N - k - 1, MOD)
    ret %= MOD
    ret *= N_1C[k]  # ここの計算量が大きい

    return ret % MOD


ans = 0
for i in range(0, K + 1):
    ans += ret_ans_when_k(i)
    ans %= MOD
print(ans)
