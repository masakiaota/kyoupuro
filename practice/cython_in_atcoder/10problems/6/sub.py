mycode = r'''
# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

# define 
ctypedef long long LL
ctypedef LL[:] Arr


import numpy as np

# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち


cdef LL a_int(): return int(readline())

def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

cdef LL N=a_int()
cdef Arr A = ints() #memoryviewはnumpyの引数が使えます
A=np.sort(A)[::-1] 
print(np.sum(A[0::2]) - np.sum(A[1::2])) #なんとmemoryviewでもスライスが使える

'''

import sys
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    import os
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
