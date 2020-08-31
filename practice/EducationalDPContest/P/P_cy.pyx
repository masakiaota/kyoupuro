# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

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


cdef LL MOD = 10**9 + 7
cdef LL INF= 9_223_372_036_854_775_807 #LLのmax

# default import
from collections import defaultdict

'''
根付き木を考える

dp[u,s] ... ノードuが色s(1が黒)のとき、u以下の部分木における色の組み合わせの総数

更新式
dp[u,1] = \prod_{c \in u.children} dp[c,0]
dp[u,0] = \prod_{c \in u.children} (dp[c,0]+dp[c,1])

末端ノードは1通り
'''

cdef LL i,j,k,_
cdef N = a_int()
XY=read_matrix(N-1,2)-1
tree = defaultdict(lambda: [])
tree = [[] for _ in range(N)]
cdef LL x,y
for x,y in XY:
    tree[x].append(y)
    tree[y].append(x)

cdef LL root=0
_dp = full((N,2),1)
cdef LL[:, ::1] dp=_dp

cdef void dfs(LL u, LL p): #pは親
    cdef LL c
    for c in tree[u]:
        if c==p:
            continue
        dfs(c,u) # 部分木のdpは先に終える必要がある
        dp[u,1] *= dp[c,0]
        dp[u,1] %= MOD
        dp[u,0] *= (dp[c,1]+dp[c,0])
        dp[u,0] %= MOD

dfs(root,-1)
print((dp[root,0]+dp[root,1])%MOD)


