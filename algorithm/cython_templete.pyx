# cython: language_level=3, boundscheck=False, wraparound=False

ctypedef long long LL

cdef chmin(LL *x, LL y):
    '''使用例 chmin(&dp[i + 1,jv], dp[i,j] +W[i])'''
    if y<x[0]:
        x[0]=y

cdef chmax(LL *x,LL y):
    '''使用例 chmax(&dp[i + 1,jv], dp[i,j] +W[i])'''
    if x[0]<y:
        x[0]=y

import numpy as np
from functools import partial
array=partial(np.array, dtype=np.int64)
zeros=partial(np.zeros, dtype=np.int64)
full=partial(np.full, dtype=np.int64)