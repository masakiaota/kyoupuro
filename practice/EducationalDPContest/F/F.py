import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.buffer.readline

'''
dp[i,j]=部分文字列が一致する最長 s[:i],t[:j]文字目まで考慮したとき

chmax(dp[i+1,j],dp[i,j])
chmax(dp[i,j+1],dp[i,j])
if s[i]==t[j]:
    chmax(dp[i+1,j+1],dp[i,j]+1)

完成したdpテーブルの終端からからやればおk
s[i]とv[j]が同じ文字だったら斜め上(i-1,j-1)にもどってそれ以外はdp[i-1,j]ordp[i,j-1]のうちにdp[i,j]と値が同じ方に戻ることにする
'''

import numpy as np
from functools import partial
array = partial(np.array, dtype=np.int64)
zeros = partial(np.zeros, dtype=np.int64)
full = partial(np.full, dtype=np.int64)

S = array(list(read()[:-1]))
T = array(list(read()[:-1]))

from numba import njit


@njit('(i8[:],i8[:],)', cache=True)
def solve(S, T):
    s = len(S)
    t = len(T)
    dp = np.zeros((len(S) + 1, len(T) + 1))
    for i in range(1, s + 1):
        for j in range(1, t + 1):
            dp[i, j] = max(dp[i, j], dp[i - 1, j])
            dp[i, j] = max(dp[i, j], dp[i, j - 1])
            if S[i - 1] == T[j - 1]:
                dp[i, j] = max(dp[i, j], dp[i - 1, j - 1] + 1)
    # 以下復元
    i, j = s, t
    ans = []
    while dp[i, j] != 0:
        if S[i - 1] == T[j - 1]:
            ans.append(S[i - 1])
            i -= 1
            j -= 1
            continue
        if dp[i, j] == dp[i - 1, j]:
            i -= 1
        else:
            j -= 1
    return ans


ret = solve(S, T)
print(''.join(map(chr, ret[::-1])))
