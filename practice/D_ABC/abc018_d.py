# https://atcoder.jp/contests/abc018/tasks/abc018_3
# わからなかったー
# マンハッタン距離 多点最短距離 動的計画法

# 菱型の中心点pと任意の点qに対してマンハッタン距離dist(p,q)<Kを満たす範囲に黒はない。
# つまり、黒点を0として各白点に対するマンハッタン距離を計算すれば、dist[p]>=Kとなるpの数が答えである。
# 端は黒で塗りつぶしておくと便利そう

import sys
read = sys.stdin.readline
INF = 10**9


def read_ints():
    return list(map(int, read().split()))


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([0 if s == 'x' else INF for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


import numpy as np
R, C, K = read_ints()
S = read_map_as_int(R)
S = np.array(S)
# xには0,oにはINFを入れておく。あとでdpするため
dp = np.zeros((R + 2, C + 2), dtype='int')
dp[1:-1, 1:-1] = S

# 上下左右に多点最小マンハッタン距離のdp(正式名称不明)を適応
for i in range(dp.shape[0] - 1):
    np.minimum(dp[i] + 1, dp[i + 1], out=dp[i + 1])
for i in range(dp.shape[0] - 1, 0, -1):
    np.minimum(dp[i] + 1, dp[i - 1], out=dp[i - 1])
for j in range(dp.shape[1] - 1):
    np.minimum(dp[:, j] + 1, dp[:, j + 1], out=dp[:, j + 1])
for j in range(dp.shape[1] - 1, 0, -1):
    np.minimum(dp[:, j] + 1, dp[:, j - 1], out=dp[:, j - 1])

# print(dp)
print((dp >= K).sum())
