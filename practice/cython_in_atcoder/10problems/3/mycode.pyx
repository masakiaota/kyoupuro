# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

# cythonライブラリよみこみ
from libcpp.vector cimport vector as Vec

# define 
ctypedef long long LL
ctypedef Vec[LL] VLL #vector[long long]
ctypedef LL[::1] Arr

import numpy as np


# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち

cdef LL a_int(): return int(readline())

def ints(): return np.fromstring(readline(), sep=' ', dtype=np.int64)

cdef LL N=a_int()
#この問題ではnumpyも使える柔軟さを紹介しよう
# 一括に処理する際は生のnumpyで良いが、indexアクセスに関しては非常に低速である。
# そういうときはtyped memoryviewという形式にする。
# といっても実態はnumpyと同じなので、numpyの関数の引数にする事ができる。ただし、ファンシーインデックスとメソッドの利用はできない。

cdef Arr A = ints() 
# せっかくなのでAの各要素は何回割れるか記録しておこう
cdef VLL candi =VLL()
cdef LL cnt, a, i
#for a in A: #この書き方はかなり遅くなるので注意が必要 #Cっぽい書き方が基本的に早いことを押さえていれば大丈夫
for i in range(len(A)):
    a=A[i]
    cnt=0
    while a%2==0: #まだ2で割れるなら
        a//=2
        cnt+=1
    candi.push_back(cnt)
#print(candi) #vectorをpython objectとして扱う場合、cythonは自動的にlistとして変換してくれる
print(min(candi))
