ctypedef long long LL


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
read = sys.stdin.buffer.readline

cdef ints(): return list(map(int, read().split()))


cdef LL i,j,k,_

cdef LL R, C, K
R, C, K=ints()

cdef LL[:,:] V=zeros((R+1,C+1))

cdef LL r,c,v
for _ in range(K):
    r,c,v=ints()
    V[r-1][c-1]=v

#dpする
cdef LL[:,:,:] dp=zeros((R+1,C+1,4))
#dp[0,0,0]=0
dp[0,0,1] = V[0,0]

# 配るDP
for i in range(R):
    for j in range(C):
        for k in range(4):
            # 拾わない場合
            chmax(&dp[i,j+1,k], dp[i,j,k]) #横に配る
            chmax(&dp[i+1,j,0], dp[i,j,k])# 縦に配る
            # 拾う場合
            chmax(&dp[i+1,j,1], dp[i,j,k]+V[i+1,j]) #縦に拾う
        for k in range(3):
            chmax(&dp[i,j+1,k+1], dp[i,j,k]+V[i,j+1]) #横に拾う

# print(array(dp).transpose(2,0,1))
# print(array(dp).shape)

print(max(dp[R - 1,C - 1]))

