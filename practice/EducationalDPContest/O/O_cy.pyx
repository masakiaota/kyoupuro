# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
ctypedef long long LL


import numpy as np
from functools import partial
array=partial(np.array, dtype=np.int64)
zeros=partial(np.zeros, dtype=np.int64)
full=partial(np.full, dtype=np.int64)

import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち


cdef LL a_int(): return int(readline())



cdef read_matrix(LL H,LL W):
    '''return np.ndarray shape=(H,W) matrix'''
    lines=[]
    cdef LL _
    for _ in range(H): 
        lines.append(read())
    lines=' '.join(lines) #byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ',dtype=np.int64).reshape(H,W)

cdef extern int __builtin_popcount(unsigned int) nogil


cdef LL MOD = 10**9 + 7

cdef LL N=a_int()
cdef LL[:,::1] A=read_matrix(N,N)

cdef LL[::1] dp = zeros((1 << N))
dp[0] = 1
cdef LL s, j
for s in range(1 << N):
    for j in range(N):
        if (s >> j) & 1 == 0 and A[__builtin_popcount(s), j]:
            dp[s + (1 << j)] += dp[s] %MOD
print(dp[(1 << N) - 1] % MOD)