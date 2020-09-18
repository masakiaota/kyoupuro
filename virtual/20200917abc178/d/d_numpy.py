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
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


S = a_int()

'''
dp[i,j] ... 問題文の条件を満たし和がiとなる数列の通りの総数。j回前に仕切りを立てた場合の。(j=2のときは2回前以前を示すことにする)

更新

# iに仕切りを立てるとき
dp[i, 0] += dp[i-1, 2] #ここから仕切りが立てられる

# iに仕切りを立てないとき
dp[i, 0] += 0 #立てないんだから
dp[i, 1] += dp[i-1,0]
dp[i, 2] += dp[i-1,1] + dp[i-1,2]
'''


from numba import njit


@njit('(i8,)', cache=True)
def solve(S):
    dp = np.zeros((S + 1, 3), dtype=np.int64)
    dp[0, 0] = 1
    for i in range(1, S + 1):
        dp[i, 0] += dp[i - 1, 2] % MOD
        dp[i, 1] += dp[i - 1, 0] % MOD
        dp[i, 2] += (dp[i - 1, 1] + dp[i - 1, 2]) % MOD

    print(dp[S, 0] % MOD)


solve(S)
