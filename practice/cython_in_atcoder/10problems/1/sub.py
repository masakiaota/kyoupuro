mycode = r'''
# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True

# define 
ctypedef long long LL

# 読み込みはpython側で行う
import numpy as np
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち


def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

cdef LL a,b
a,b=ints()
print('Odd' if (a*b)&1 else 'Even')
'''

import sys
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    import os
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
