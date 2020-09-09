mycode = r'''
# distutils: language=c++
# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True

ctypedef long long LL
cdef LL MOD = 10**9 + 7
cdef LL N = int(input())
cdef LL ans = 1

cdef i
for i in range(2, N + 1):
    ans *= i
    ans %= MOD
print(ans)
'''

import sys
import os
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
