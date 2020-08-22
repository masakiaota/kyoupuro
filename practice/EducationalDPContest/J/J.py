import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち

import numpy as np
from functools import partial
array = partial(np.array, dtype=np.int64)
zeros = partial(np.zeros, dtype=np.int64)
full = partial(np.full, dtype=np.int64)

ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


def read_matrix(H, W):
    '''return np.ndarray shape=(H,W) matrix'''
    lines = []
    for _ in range(H):
        lines.append(read())
    lines = ' '.join(lines)  # byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ', dtype=np.int64).reshape(H, W)


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import

from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce, lru_cache
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


N = a_int()
A = ints()
cnt = Counter(A)
'''
dp[i,j,k]...すべての寿司がなくなるまでの操作の期待値(寿司が1つ乗っている皿の数がi,2つがj,3つがk)

dp[i,j,k]= 1(操作回数)+ 
            i/N*(1つの皿が一つ減った場合の操作の期待値) + 
            j/N*(2つの皿が1つ減った場合の操作の期待値) + 
            k/N*(3つの皿が1つ減った場合の操作の期待値)+
            (1-i-j-k)/N * (0つの皿を選んでしまったときの期待値)

dp[i,j,k]= 1+ i/N * dp[i-1,j,k]+ 
            j/N*dp[i+1,j-1,k] + 
            k/N*dp[i,j+1,k-1]+
            (N-i-j-k)/N*dp[i,j,k]

dp[i,j,k]= N/(i+j+k) *
            (1+ i/N * dp[i-1,j,k]+ 
            j/N*dp[i+1,j-1,k] + 
            k/N*dp[i,j+1,k-1])
'''

dp = np.full((N + 1, N + 1, N + 1), -1.0)
dp[0, 0, 0] = 0
from numba import njit
# @njit('(i8,i8,i8[:])',cache=True)


@njit('(i8,i8,i8,i8,f8[:,:,:])', cache=True)
def dfs(i, j, k, N, dp):
    if i < 0 or j < 0 or k < 0 or i == j == k == 0:
        return 0
    if dp[i, j, k] != -1:
        return dp[i, j, k]
    ret = 1
    ret += i / N * dfs(i - 1, j, k, N, dp)
    ret += j / N * dfs(i + 1, j - 1, k, N, dp)
    ret += k / N * dfs(i, j + 1, k - 1, N, dp)
    if (i + j + k) == 0:
        dp[i, j, k] = ret
        return ret
    dp[i, j, k] = N / (i + j + k) * ret
    return dp[i, j, k]


print(dfs(cnt[1], cnt[2], cnt[3], N, dp))
