mycode = r'''
# distutils: language=c++
# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

ctypedef long long LL
from libc.stdio cimport scanf, printf

cdef LL a,b,c,_,i
cdef LL[3][3] C
for i in range(3):
    scanf('%lld %lld %lld', &C[i][0],&C[i][1],&C[i][2])

    
cdef LL a0,a1,a2,b0,b1,b2
cdef bint flg=0
for a0 in range(101):
    b0=C[0][0]-a0
    b1=C[0][1]-a0
    b2=C[0][2]-a0
    for a1 in range(101):
        for a2 in range(101):
            if b0 == C[1][0]-a1 and b1==C[1][1]-a1 and b2==C[1][2]-a1 and b0 == C[2][0]-a2 and b1==C[2][1]-a2 and b2==C[2][2]-a2:
                if flg==0:
                   printf("Yes")
                flg=1
if flg==0:
    printf('No')

'''

import sys
import os
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
