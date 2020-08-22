# cython: language_level=3, boundscheck=False, wraparound=False

ctypedef long long LL

cdef chmin(LL *x, LL y):
    '''使用例 chmin(&dp[i + 1,jv], dp[i,j] +W[i])'''
    if y<x[0]:
        x[0]=y

cdef chmax(LL *x,LL y):
    '''使用例 chmax(&dp[i + 1,jv], dp[i,j] +W[i])'''
    if x[0]<y:
        x[0]=y

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

cdef ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

def py_ints() : list(map(int, readline().split()))

cdef read_matrix(LL H,LL W):
    '''return np.ndarray shape=(H,W) matrix'''
    lines=[]
    cdef LL _
    for _ in range(H): 
        lines.append(read())
    lines=' '.join(lines) #byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ',dtype=np.int64).reshape(H,W)

def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))

def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret



cdef LL MOD = 10**9 + 7
cdef LL INF= 9_223_372_036_854_775_807 #LLのmax

# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from bisect import bisect_left, bisect_right #, insort_left, insort_right
from functools import reduce
from math import gcd
