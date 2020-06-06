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
INF = 2**63  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# DPっぽい
N, L = read_ints()
X = read_ints()
X.reverse()

T1, T2, T3 = read_ints()
'''
dp[i] ... 座標iにいるときの最短時間
更新則
dp[i] = min(dp[i-1]+T1, dp[i-2]+T1+T2, dp[i-4]+T1+3T2)
ただしiにハードルがあった場合は+T3
'''
dp = [INF] * (L + 1)
dp[0] = 0

for i in range(1, L + 1):
    dp[i] = min(dp[i], dp[i - 1] + T1)
    if i - 2 >= 0:
        dp[i] = min(dp[i], dp[i - 2] + T1 + T2)
    if i - 4 >= 0:
        dp[i] = min(dp[i], dp[i - 4] + T1 + 3 * T2)
    if X and X[-1] == i:
        dp[i] += T3
        del X[-1]

ans = INF
ans = min(ans, dp[-1])
ans = min(ans, dp[-2] + T1 // 2 + T2 // 2)  # ゴールから-1の地点からジャンプしてゴール(行動2and3)
ans = min(ans, dp[-3] + T1 // 2 + T2 + T2 // 2)  # ゴールから-2の地点からジャンプしてゴール(行動3)
# ゴールから-2の地点からジャンプしてゴール(行動3)
if L != 2:
    ans = min(ans, dp[-4] + T1 // 2 + 2 * T2 + T2 // 2)
print(ans)
# Lをジャンプして通過する可能性があるのか
