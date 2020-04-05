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


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a * b // g


N, Q = read_ints()
A = read_ints()
S = read_ints()

# gcd(A)を始めにしておけば、Siに対するGCDも即座に求まる
# 問題はX=1であるようなj
# gcdを累積して二分探索を行えば良い
gcd_cum = []  # 累積gcd配列
g = 0
for a in A:
    g = gcd(g, a)
    gcd_cum.append(g)


def is_ok(arg, s):
    # arg(idx)が与えられたときに、gcd(gcd_cum[idx],s)==1を満たす最小のidxがほしい
    return gcd(gcd_cum[arg], s) == 1


def meguru_bisect(ng, ok, s):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid, s):
            ok = mid
        else:
            ng = mid
    return ok


for s in S:
    ans = gcd(g, s)
    print(ans if ans != 1 else (meguru_bisect(-1, N, s) + 1))
