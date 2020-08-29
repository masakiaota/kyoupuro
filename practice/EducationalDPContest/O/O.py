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


def read_matrix(H, W):
    '''return np.ndarray shape=(H,W) matrix'''
    lines = []
    for _ in range(H):
        lines.append(read())
    lines = ' '.join(lines)  # byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ', dtype=np.int64).reshape(H, W)


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import

from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce

N = a_int()
A = read_matrix(N, N)

'''
一方の集合を管理してDPというアイデア

dp[i][s] ... 男性を:i人目まで見たときに、マッチングした女性の集合がsであるような通りの総数

更新式
dp[i+1][s+1<<j] += dp[i][s] for s のjbit目は0 かつ A[i,j]==1

これでは計算量がO(2^N 2N)なのでTLE
iと|s|は同じなのだから状態をまとめることができる
(二次元dpでいうとi=|s|のセルでしか状態遷移が発生しない)

よって単純sだけでループしても良さそうだし、()内のアイデアで枝刈りしても良さそう
'''

from numba import njit


@njit('(i8,i8[:,:])', cache=True)
def solve_TLE(N, A):
    dp = np.zeros((N + 1, 1 << N), dtype=np.int64)
    dp[0, 0] = 1
    for i in range(N):
        for s in range(1 << N):
            for j in range(N):
                if (s >> j) & 1 == 0 and A[i, j] == 1:
                    dp[i + 1, s + (1 << j)] += dp[i, s]
                    dp[i + 1, s + (1 << j)] %= MOD
    print(dp[N, (1 << N) - 1] % MOD)


@njit('i8(i8)', cache=True)
def popcount(x):
    '''xの立っているビット数をカウントする関数
    (xは64bit整数)'''

    # 2bitごとの組に分け、立っているビット数を2bitで表現する
    x = x - ((x >> 1) & 0x5555555555555555)

    # 4bit整数に 上位2bit + 下位2bit を計算した値を入れる
    x = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333)

    x = (x + (x >> 4)) & 0x0f0f0f0f0f0f0f0f  # 8bitごと
    x = x + (x >> 8)  # 16bitごと
    x = x + (x >> 16)  # 32bitごと
    x = x + (x >> 32)  # 64bitごと = 全部の合計
    return x & 0x0000007f


@njit('(i8,i8[:,:])', cache=True)
def solve(N, A):  # 遷移のi==|s|の遷移しか生じないことに注目すれば
    dp = np.zeros((N + 1, 1 << N), dtype=np.int64)
    dp[0, 0] = 1
    for i in range(N):
        for s in range(1 << N):
            if popcount(s) != i:  # 自分的にはこっちの枝刈りのほうがしっくり
                continue
            # if dp[i, s] == 0:  # 遷移するものがない
            #     continue
            for j in range(N):
                if (s >> j) & 1 == 0 and A[i, j] == 1:
                    dp[i + 1, s + (1 << j)] += dp[i, s]
                    dp[i + 1, s + (1 << j)] %= MOD
    print(dp[N, (1 << N) - 1] % MOD)


@njit('(i8,i8[:,:])', cache=True)
def solve2(N, A):  # iの代わりにsから情報を得る #s1 \subset s2 ならばs1<s2が成り立つので小さい方からforすればdpが成り立つ
    dp = np.zeros((1 << N), dtype=np.int64)
    dp[0] = 1
    for s in range(1 << N):
        for j in range(N):
            if (s >> j) & 1 == 0 and A[popcount(s), j] == 1:
                dp[s + (1 << j)] += dp[s]
                dp[s + (1 << j)] %= MOD
    print(dp[(1 << N) - 1] % MOD)


# solve2(N, A)
solve(N, A)
