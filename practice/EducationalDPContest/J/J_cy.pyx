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

import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち

def exit(*argv,**kwarg):
    print(*argv,**kwarg)
    sys.exit()


cdef LL a_int(): return int(readline())

cdef ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


cdef LL MOD = 10**9 + 7
cdef LL INF= 9_223_372_036_854_775_807 #LLのmax

# default import
from collections import Counter

cdef LL N = a_int()
A = ints()
cnt = Counter(A)

'''
dp[i,j,k]...すべての寿司がなくなるまでの操作の期待値(寿司が1つ乗っている皿の数がi,2つがj,3つがk)

dp[i,j,k]= 1(操作回数)+ 
            i/N*(1つの皿が一つ減った場合の操作の期待値) + 
            j/N*(2つの皿が1つ減った場合の操作の期待値) + 
            k/N*(3つの皿が1つ減った場合の操作の期待値)+
            (1-i-j-k)/N * (0つの皿を選んでしまったときの期待値)

dp[i,j,k]= 1+ i/N * dp[i-1,j,k]+ 
            j/N*dp[i+1,j-1,k] + 
            k/N*dp[i,j+1,k-1]+
            (N-i-j-k)/N*dp[i,j,k]

dp[i,j,k]= N/(i+j+k) *
            (1+ i/N * dp[i-1,j,k]+ 
            j/N*dp[i+1,j-1,k] + 
            k/N*dp[i,j+1,k-1])
'''

cdef double[:,:,::1] dp = np.full((N + 1, N + 1, N + 1), -1.0)
dp[0, 0, 0] = 0.0
cdef double dfs(LL i, LL j, LL k,LL N):
    if i < 0 or j < 0 or k < 0:
        return 0
    if dp[i, j, k] != -1:
        return dp[i, j, k]
    cdef double ret = 1
    ret += i / N * dfs(i - 1, j, k, N)
    ret += j / N * dfs(i + 1, j - 1, k, N)
    ret += k / N * dfs(i, j + 1, k - 1, N)
    if (i + j + k) == 0:
        dp[i, j, k] = ret
        return ret
    dp[i, j, k] = N / (i + j + k) * ret
    return dp[i, j, k]


print(dfs(<LL>cnt[1], <LL>cnt[2], <LL>cnt[3], N))
