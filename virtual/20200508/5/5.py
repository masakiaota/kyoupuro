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

# https://atcoder.jp/contests/abc135/tasks/abc135_d
# 5あまるじゃなくてn余るできないかな？

'''
dp[i][j] ... [0,i)桁の数字で、余りがjに成る通りの総数
更新則
?じゃないとき
dp[i+1][(j+S[i]*pow(10,i,13))%13] = dp[i][j]
?のとき
dp[i+1][j] += dp[i][k] #すべてのjとkで
'''
mod = 13
S = list(reversed(read()[:-1]))

dp = [[0] * mod for _ in range(len(S) + 1)]
dp[0][0] = 1
pow10imod = 1
for i in range(len(S)):
    for j in range(mod):
        if S[i] != '?':
            dp[i + 1][(j + int(S[i]) * pow10imod) % mod] += \
                dp[i][j] % MOD
        else:
            for k in range(10):
                dp[i + 1][(j + k * pow10imod) % mod] += \
                    dp[i][j] % MOD
    pow10imod = pow10imod * 10 % mod

print(dp[-1][5] % MOD)
# print(*dp, sep='\n')
