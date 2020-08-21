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
read = sys.stdin.readline  # 文字列読み込む時はこっち



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

cdef LL MOD = 10**9 + 7

import numpy as np
cdef LL i,j,_
cdef LL H,W
H, W = ints()
cdef LL[:,:] A = array(read_map_as(H))
cdef LL[:,:] dp = zeros((H, W))
dp[0,0]=1
for i in range(H):
    for j in range(W):
        if A[i,j]==0:
            dp[i, j]=0
            continue
        if i>0:
            dp[i,j]+=dp[i-1,j]
        if j>0:
            dp[i,j]+=dp[i,j-1]
        dp[i,j]=dp[i,j]%MOD
print(dp[-1,-1])
