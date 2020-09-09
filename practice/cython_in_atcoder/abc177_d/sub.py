mycode = r'''
# distutils: language=c++
# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

ctypedef long long LL
from libc.stdio cimport scanf, printf
from libcpp.vector cimport vector
ctypedef vector[LL] vec


cdef class UnionFind:
    cdef:
        LL N,n_groups
        vec parent

    def __init__(self, LL N):
        self.N = N  # ノード数
        self.n_groups = N  # グループ数
        # 親ノードをしめす。負は自身が親ということ。
        self.parent = vec(N,-1)  # idxが各ノードに対応。

    cdef LL root(self, LL A):
        # print(A)
        # ノード番号を受け取って一番上の親ノードの番号を帰す
        if (self.parent[A] < 0):
            return A
        self.parent[A] = self.root(self.parent[A])  # 経由したノードすべての親を上書き
        return self.parent[A]

    cdef LL size(self, LL A):
        # ノード番号を受け取って、そのノードが含まれている集合のサイズを返す。
        return -self.parent[self.root(A)]

    cdef bint unite(self,LL A,LL B):
        # ノード番号を2つ受け取って、そのノード同士をつなげる処理を行う。
        # 引数のノードを直接つなぐ代わりに、親同士を連結する処理にする。
        A = self.root(A)
        B = self.root(B)

        # すでにくっついている場合
        if (A == B):
            return False

        # 大きい方に小さい方をくっつけたほうが処理が軽いので大小比較
        if (self.size(A) < self.size(B)):
            A, B = B, A

        # くっつける
        self.parent[A] += self.parent[B]  # sizeの更新
        self.parent[B] = A  # self.rootが呼び出されればBにくっついてるノードもすべて親がAだと上書きされる
        self.n_groups -= 1

        return True

    cdef bint is_in_same(self,LL A,LL B):
        return self.root(A) == self.root(B)


cdef LL N,M,_
scanf('%lld %lld',&N, &M)

cdef UnionFind uf = UnionFind(N)
cdef LL a,b
for _ in range(M):
    scanf('%lld %lld',&a, &b)
    uf.unite(a-1, b-1)

print(-min(uf.parent))
'''

import sys
import os
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
