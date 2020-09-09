

import sys
import os
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    mycode = r'''
# distutils: language=c++
# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

ctypedef long long LL
from libc.stdio cimport scanf
from libcpp.vector cimport vector as vec

ctypedef vec[vec[LL]] Graph
cdef LL i,j,k,_

cdef LL N,M
scanf('%lld %lld',&N,&M)

cdef Graph graph=Graph(N)

cdef LL a,b

# この問題をあえてグラフの構造を持つことで解く
for _ in range(M):
    scanf('%lld %lld',&a, &b)
    graph[a-1].push_back(b-1)
    graph[b-1].push_back(a-1)

cdef vec[LL] ans
for i in range(N):
    ans.push_back(graph[i].size())

print(*ans, sep='\n')

'''
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
