# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

# cythonライブラリよみこみ
cimport cython
from cython.operator cimport typeid #typeid(x).name()で一応型を確認できるっぽい？
from libcpp cimport bool
from libcpp.vector cimport vector as Vec
from libcpp.deque cimport deque as Deque
from libcpp.unordered_map cimport unordered_map as Map
from libcpp.unordered_set cimport unordered_set as Set
from libcpp.pair cimport pair as Pair
from libcpp.string cimport string as Str
from libcpp.queue cimport priority_queue as PriorityQueue
from libcpp.typeinfo cimport type_info #type確認
cimport libc.math as cmath

cdef extern from "<algorithm>" namespace "std":
    void swap[T](T& a, T& b) except +  # array overload also works

cdef extern int __builtin_popcount(unsigned int) nogil #bitの数
# TODO gcdとかも外部から取ってきたい


# define 
ctypedef long long LL
ctypedef long double LD
ctypedef fused real:
    LL
    LD
ctypedef Vec[LL] VLL #vector[long long]
ctypedef Vec[LD] VLD #vector[long double]
ctypedef LL[:] Arr
ctypedef LL[:,:] Arr2d
ctypedef LL[:,:,:] Arr3d
ctypedef LL[:,:,:,:] Arr4d #ちなみに7次元までサポートしてる

# cythonの関数定義
cdef chmin(real& x, real y):
    #使用例 chmin(dp[i + 1,jv], dp[i,j] +W[i])
    cdef real* p= &x
    if y<x: p[0]=y

cdef chmax(real& x,real y):
    #使用例 chmax(&dp[i + 1,jv], dp[i,j] +W[i])
    cdef real* p= &x
    if x<y: p[0]=y


import numpy as np
from functools import partial
array=partial(np.array, dtype=np.int64)
zeros=partial(np.zeros, dtype=np.int64)
full=partial(np.full, dtype=np.int64)

# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち

def exit(*argv,**kwarg):
    print(*argv,**kwarg)
    sys.exit()


cdef LL a_int(): return int(readline())

def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)
def doubles(): return np.fromstring(readline(), sep=' ', dtype=np.longdouble)

def py_ints() : list(map(int, readline().split()))

cdef read_matrix(LL H,LL W):
    #return np.ndarray shape=(H,W) matrix
    lines=[]
    cdef LL _
    for _ in range(H): 
        lines.append(read())
    lines=' '.join(lines) #byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ',dtype=np.int64).reshape(H,W)


cdef LL MOD = 10**9 + 7
cdef LL INF= 9_223_372_036_854_775_807 #LLのmax
cdef LL i,j,k,_

# python import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from bisect import bisect_left, bisect_right #, insort_left, insort_right
from functools import reduce
from math import gcd
