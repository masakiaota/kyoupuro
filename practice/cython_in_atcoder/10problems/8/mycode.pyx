# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

ctypedef long long LL

import numpy as np

# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち

def exit(*argv,**kwarg):
    print(*argv,**kwarg)
    sys.exit()


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

cdef LL i,j,k,_

cdef LL N,Y
N,Y=ints()
Y//=1000

cdef LL a,b,c #a+b+c=Nかつ10a+5b+c=Yを満たすa,b,cを見つけたい
for a in range(N+1):
    for b in range(N+1):
        c = N-a-b
        if c<0 : break
        if 10*a+5*b+c==Y:
            exit(a,b,c)
print(-1,-1,-1)
        

