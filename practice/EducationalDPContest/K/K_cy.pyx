# cython: language_level=3, boundscheck=False, wraparound=False

ctypedef long long LL

import numpy as np
from functools import partial
array=partial(np.array, dtype=np.int64)
zeros=partial(np.zeros, dtype=np.int64)
full=partial(np.full, dtype=np.int64)

import sys
readline = sys.stdin.buffer.readline


cdef ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

# https://atcoder.jp/contests/abc176/tasks

'''
dp[i]...次に行動するプレイヤーが勝つかどうか。石がi個あったときに。

dp[i]=True if (i-a for a \in A で一つでもFalseがあったら) else False
これは配るdpを用いて
dp[i+a] |= 1-dp[i] で書くことができる
'''

cdef LL N,K,i,a
N, K = ints()
cdef LL[::1] A =ints()
cdef LL[::1] dp =zeros((K+max(A)+1))
for i in range(K):
    for j in range(N):
        dp[i + A[j]] |= 1 - dp[i]
print('First' if dp[K] else 'Second')
