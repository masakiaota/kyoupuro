# https://atcoder.jp/contests/abc032/tasks/abc032_d

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9 #63乗にしたらTLE...
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N, K = read_ints()
V, W = read_col(N)


# 制約をうまく使う→制約はNが小さいかwが小さいかvが小さいと言っている 小さいのを最小する実装をすれば良い
# Nが小さい時→半分全列挙の巨大ナップサック
# Wが小さい時→dp[i][w]の動的計画法
# Vが小さい時→dp[i][v]の動的計画法

def solver1(V, W):
    '''半分全列挙の巨大ナップサック'''
    N_half = N // 2
    WV1 = defaultdict(lambda: -1)  # w_sumをkeyにその重さの最大価値をvalueにする
    for bit in product(range(2), repeat=N_half):
        w_sum = v_sum = 0
        for i, to_use in enumerate(bit):
            if to_use:
                w_sum += W[i]
                v_sum += V[i]
        if w_sum <= K:
            WV1[w_sum] = max(WV1[w_sum], v_sum)

    WV2 = defaultdict(lambda: -1)
    for bit in product(range(2), repeat=N - N_half):
        w_sum = v_sum = 0
        for i, to_use in enumerate(bit, start=N_half):
            if to_use:
                w_sum += W[i]
                v_sum += V[i]
        if w_sum <= K:
            WV2[w_sum] = max(WV2[w_sum], v_sum)
    # WV2についてwもvについても単調増加にする (重くて価値のないものは明らかに無駄)
    W2, V2 = [], []
    ma = -1
    for w, v in sorted(WV2.items()):
        if ma < v:
            W2.append(w)
            V2.append(v)
            ma = v
    # 二分探索で答えを見つける
    ans = 0
    for w, v in WV1.items():
        idx = bisect_right(W2, K - w) - 1
        ans = max(ans, V2[idx] + v)
    print(ans)


def solver2(V, W):
    '''wについてナップサックdp
    dp[i][j]...重さj以下になる価値の最大。[,i)の荷物を考慮したとき。'''
    w_max = min(sum(W), K)
    dp = [[0] * (w_max + 1) for _ in range(N + 1)]
    for i, j in product(range(N), range(1, w_max + 1)):
        dp[i + 1][j] = max(dp[i + 1][j], dp[i][j])
        if j - W[i] >= 0:
            dp[i + 1][j] = max(dp[i + 1][j], dp[i][j - W[i]] + V[i])
    print(dp[-1][-1])


def solver3(V, W):
    '''vについてナップサックdp
    dp[i][j]...価値がちょうどjになる重さの最小。[,i)の荷物を考慮したとき'''
    v_max = sum(V)
    dp = [[INF] * (v_max + 1) for _ in range(N + 1)]
    for i in range(N + 1):
        dp[i][0] = 0  # 価値が0になる重さは0

    for i, j in product(range(N), range(1, v_max + 1)):
        dp[i + 1][j] = min(dp[i + 1][j], dp[i][j])
        if j - V[i] >= 0:
            dp[i + 1][j] = min(dp[i + 1][j], dp[i][j - V[i]] + W[i])

    # 左から重さがK以下の要素となるjを探す(価値の最大)
    for j in range(v_max, -1, -1):
        if dp[-1][j] <= K:
            print(j)
            break


if N <= 30:
    solver1(V, W)
elif max(W) <= 1000:
    solver2(V, W)
elif max(V) <= 1000:
    solver3(V, W)
