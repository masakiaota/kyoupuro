# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

ctypedef long long LL


cdef chmax(LL *x,LL y):
    '''使用例 chmax(&dp[i + 1,jv], dp[i,j] +W[i])'''
    if x[0]<y:
        x[0]=y

cdef extern int __builtin_popcount(unsigned int) nogil

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

cdef ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


cdef i,j,k,_
cdef LL A,B
A,B=ints()
cdef LL[::1] a =ints()[::-1].copy()
cdef LL[::1] b =ints()[::-1].copy()

_a_cum=zeros(A+1)
_a_cum[1:]=np.cumsum(a)
cdef LL[::1] a_cum=_a_cum
_b_cum=zeros(B+1)
_b_cum[1:]=np.cumsum(b)
cdef LL[::1] b_cum=_b_cum
cdef LL tot(LL i,LL j): #a[:i],b[:j]の合計
    return a_cum[i] + b_cum[j]

_dp=zeros((A+1,B+1))
cdef LL[:,::1] dp=_dp
for i in range(A+1):
    for j in range(B+1):
        if i+1<A+1:
            chmax(&dp[i+1, j], tot(i,j)-dp[i,j]+a[i])
        if j+1<B+1:
            chmax(&dp[i, j+1], tot(i,j)-dp[i,j]+b[j])
print(dp[A,B])
# print(_dp)
