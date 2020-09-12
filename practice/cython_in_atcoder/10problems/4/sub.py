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

# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline
read = sys.stdin.readline #文字列読み込む時はこっち

cdef LL a_int(): return int(readline())

cdef LL i,j,k,_


A=a_int()
B=a_int()
C=a_int()
X=a_int() #あえて型をつけない(→実装が早くなる)

# 愚直にシミュレーション
cdef LL a,b,c,ans #他の定数はぶっちゃけ型を付ける必要がないですが、ループにはしっかりつけないと速度が低下する
for a in range(A+1):
    for b in range(B+1):
        for c in range(C+1):
            ans += (500*a + 100*b + 50*c == X)
print(ans)


'''

import sys
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    import os
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
