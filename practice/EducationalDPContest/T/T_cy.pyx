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

def exit(*argv,**kwarg):
    print(*argv,**kwarg)
    sys.exit()


cdef LL a_int(): return int(readline())


cdef LL MOD = 10**9 + 7
cdef LL INF= 9_223_372_036_854_775_807 #LLのmax


cdef LL N=a_int()
S=input()

_dp=zeros((N,N))
_dp[0,:]=1
cdef LL[:,::1] dp =_dp

_dp_cum=zeros(N+1)
cdef LL[::1] dp_cum =_dp_cum

cdef LL i,j
for i in range(N-1):
    _dp_cum[1:]=np.cumsum(dp[i, :]) %MOD
    # dp_cum=_dp_cum #メモリを共有してるので自動的に書き換わる

    for j in range(N-i-1):
        if S[i]=='<': #jより小さい方を集める
            dp[i+1,j]=dp_cum[j+1]-dp_cum[0] + MOD
        else:
            dp[i+1,j]=dp_cum[N-i]-dp_cum[j+1] + MOD

#print(_dp)
print(dp[N-1,0]%MOD)
        

