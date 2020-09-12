# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

# cythonライブラリよみこみ

from libc.stdlib cimport abs as iabs

# define 
ctypedef long long LL
ctypedef LL[:] Arr

import numpy as np
from functools import partial
zeros=partial(np.zeros, dtype=np.int64)


# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち

def exit(*argv,**kwarg):
    print(*argv,**kwarg)
    sys.exit()


cdef LL a_int(): return int(readline())

cdef read_matrix(LL H,LL W):
    #return np.ndarray shape=(H,W) matrix
    lines=[]
    cdef LL _
    for _ in range(H): 
        lines.append(read())
    lines=' '.join(lines) #byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ',dtype=np.int64).reshape(H,W)

cdef LL i,j,k,_


# tの偶奇は市松模様の分布になる
# つまり0,0からのマンハッタン距離が偶数のとき偶数時刻を許容でき、距離が奇数のとき奇数時刻を許容できる
# またt[n]-t[n-1]がマンハッタン距離を上回る場合は到達不可能なことに気をつけよう

N=a_int()
data=zeros((N+1,3))
data[1:] = read_matrix(N,3)

cdef Arr T =data[:,0]
cdef Arr X =data[:,1]
cdef Arr Y =data[:,2]

cdef LL manhattan(LL x, LL y):
    return iabs(x)+iabs(y)

cdef LL dt, dd,
for i in range(N):
    dt=T[i+1]-T[i]
    dd=manhattan(X[i+1]-X[i], Y[i+1]-Y[i])
    if (manhattan(X[i+1],Y[i+1])&1 != T[i+1]&1) or (dd>dt):
        exit('No')
print('Yes')


