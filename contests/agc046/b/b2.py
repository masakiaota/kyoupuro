# https://atcoder.jp/contests/agc046/tasks/agc046_b
#
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def combination_mod(n, r, mod=998244353):
    # mod取りながらcombination
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


MOD = 998244353
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd

A, B, C, D = read_ints()
i_max = C - A
j_max = D - B

'''
A+i,B+jまで広げたときを考える
このときの通りの総数は

更新則
dp[i][j]=dp[i-1][j]*(B+j)+dp[i][j-1]*(A+i)
        -dp[i-1][j-1]*(A+i-1)*(B+j-1)
∵(A+i,B+j)サイズの通りの数について縦だけ考えた場合と横だけ考えた場合の和には全通り含まれている
重複分はdp[i-1][j-1]*(A+i-1)*(B+j-1)なのでこれを引く。直感的には 右下1マスがない場合の通りの総数と言える
んーうまく説明できない
'''


dp = [[0] * (j_max + 1) for _ in ra(i_max + 1)]
# 初期値代入
for i in ra(i_max + 1):
    dp[i][0] = pow(B, i, MOD)

for j in ra(j_max + 1):
    dp[0][j] = pow(A, j, MOD)

# ループ
for i, j in product(range(1, i_max + 1), range(1, j_max + 1)):

    dp[i][j] = dp[i][j - 1] * (A + i) \
        + dp[i - 1][j] * (B + j) \
        - (A + i - 1) * (B + j - 1) * dp[i - 1][j - 1]  # この重複ってどうなってるんだ？

    dp[i][j] %= MOD

print(*dp, sep='\n')
print(dp[-1][-1])
