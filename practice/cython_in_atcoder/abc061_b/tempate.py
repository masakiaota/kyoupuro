mycode = r'''
# distutils: language=c++
# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

ctypedef long long LL

cdef extern int __builtin_popcount(unsigned int) nogil

import numpy as np

import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち


cdef ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)


cdef read_matrix(LL H,LL W):
    # return np.ndarray shape=(H,W) matrix
    lines=[]
    cdef LL _
    for _ in range(H): 
        lines.append(read())
    lines=' '.join(lines) #byte同士の結合ができないのでreadlineでなくreadで
    return np.fromstring(lines, sep=' ',dtype=np.int64).reshape(H,W)

# この問題をあえてグラフの構造を持つことで解く
from libcpp.vector cimport vector as vec

cdef LL N,M
N,M=ints()
AB=read_matrix(M,2)-1
cdef LL a,b

ctypedef vec[vec[LL]] Graph
cdef Graph graph=Graph(N)

for a,b in AB:
    graph[a].push_back(b)
    graph[b].push_back(a)

cdef vec[int] ans
for i in range(N):
    ans.push_back(graph[i].size())
print(*ans, sep='\n')
'''

import sys
import os
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
