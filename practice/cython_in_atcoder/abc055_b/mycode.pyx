# distutils: language=c++
# cython: language_level=3, boundscheck=False, wraparound=False
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

from libcpp.vector cimport vector


ctypedef long long LL
cdef LL MOD = 10**9 + 7
cdef LL N = int(input())
cdef LL ans = 1
cdef vector[LL] ansls

cdef i
for i in range(2, N + 1):
    ans *= i
    ans %= MOD
    ansls.push_back(ans)
print(ans)
# print(type(ansls))
