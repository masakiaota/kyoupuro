# https://atcoder.jp/contests/abc095/tasks/arc096_b

# 嘘解法やないかい

# good noteにアイデアを書き出してる
# 重要な考察だけ
# 貪欲に次を決定する方法は取れない。∵見られない寿司が出てくる。その栄養価が高いとWA
# 最短距離を歩くにはずっと時計回りに進んでからどこかで折り返して反時計回りに進むのが最適。(逆も試してより大きな方の結果を得れば良い)
# じゃどこで折り返すのがMAX？スコアの累積のMAX(左端)でしょ

import sys
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
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations, accumulate
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


N, C = read_ints()
X, V = read_col(N)


def ret_ans(X, V):  # 片方向だけ実装する
    # 右から左に探索、取る価値のある寿司の一番右側
    V_accum = list(accumulate(V))
    ma = 0
    idx_ma = 0
    ret_ls = [-1] * N
    for j, (v, x) in enu(zip(V_accum, X)):
        ret_ls[j] = v - x
        if v - x > ma:
            ma = v - x
            idx_ma = j
    v = V[idx_ma]
    x_idx_ma = X[idx_ma]
    for i in range(N - 1, idx_ma, -1):
        v += V[i]
        y = C - X[i]
        ret_ls[j] = v - (y + 2 * x_idx_ma)
    print(idx_ma, ret_ls)
    return max(ret_ls)


ans1 = ret_ans(X, V)
V_reversed = V[::-1]
X_reversed = []
for x in reversed(X):
    X_reversed.append(C - x)
print(max(ret_ans(X_reversed, V_reversed), ans1))
