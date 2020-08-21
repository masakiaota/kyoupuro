import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def ints(): return list(map(int, readline().split()))


def read_map_as(H, replace={'#': 1, '.': 0}, pad=None):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    if pad is None:
        ret = []
        for _ in range(H):
            ret.append([replace[s] for s in read()[:-1]])
            # 内包表記はpypyでは若干遅いことに注意
            # #numpy使うだろうからこれを残しておくけど
    else:  # paddingする
        ret = [[pad] * (W + 2)]  # Wはどっかで定義しておくことに注意
        for _ in range(H):
            ret.append([pad] + [replace[s] for s in read()[:-1]] + [pad])
        ret.append([pad] * (W + 2))

    return ret


MOD = 10**9 + 7

import numpy as np
H, W = ints()
A = np.array(read_map_as(H))


from numba import njit


@njit('(i8,i8,i8[:,:])', cache=True)
def solve(H, W, A):
    dp = np.zeros((H, W), dtype=np.int64)
    dp[0, 0] = 1
    for i in range(H):
        for j in range(W):
            if A[i, j] == 0:
                if i > 0:
                    dp[i, j] += dp[i - 1, j]
                if j > 0:
                    dp[i, j] += dp[i, j - 1]
                dp[i, j] %= MOD
    print(dp[H - 1, W - 1])


solve(H, W, A)
