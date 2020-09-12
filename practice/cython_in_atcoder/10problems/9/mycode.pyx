# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: infer_types=True
# cython: cdivision=True
# False:Cython はCの型に対する除算・剰余演算子に関する仕様を、(被演算子間の符号が異なる場合の振る舞いが異なる)Pythonのintの仕様に合わせ、除算する数が0の場合にZeroDivisionErrorを送出します。この処理を行わせると、速度に 35% ぐらいのペナルティが生じます。 True:チェックを行いません。

from libcpp.string cimport string as Str
# define 
ctypedef long long LL

# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline().rstrip

cdef LL i,j,k,_


cdef Str S = readline()
cdef Str dream = b'dream'
cdef Str dreamer = b'dreamer'
cdef Str erase = b'erase'
cdef Str eraser = b'eraser'

# 文字列処理は流石にpypy提出のほうが楽そう
# 致命的な点としては、libcpp.stringがeraseなどの文字列削除操作に対応していない点だ

cdef LL idx=S.size()
while idx>7:
    if S.substr(idx-5,5)==dream: #idxから後ろ5文字以降を表示
        idx-=5 
    elif S.substr(idx-5,5)==erase:
        idx-=5 
    elif S.substr(idx-6,6)==eraser:
        idx-=6
    elif S.substr(idx-7,7)==dreamer:
        idx-=7 
    else: break

#print(S,idx,S.substr(idx))
if S.substr(0, idx) in [dream, dreamer, erase, eraser]:
    print('YES')
else:
    print('NO')

