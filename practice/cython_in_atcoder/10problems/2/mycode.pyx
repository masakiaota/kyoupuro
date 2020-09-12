# distutils: language=c++
# cython: language_level=3
# cython: boundscheck=False

from libcpp.string cimport string as Str

# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline # byte読み込み

cdef Str S = readline()[:-1] 
print(S.count(b'1'))

# ここでは練習のためにわざわざC++のstring型で格納している
# pythonのstrではなくbytes型でないとStrに格納できないので注意しよう
# この問題ではpythonのstrを用いても実行速度が変わらないが、S[i]にアクセスする必要がある場合には100倍ほど定数倍が変化する
# 実装量と相談して上手に使い分けよう





## cython
#cdef LL n = 10**7
#cdef Str S = readline()[:-1] * n 
##S = readline()[:-1] * n 
#S[4]=b'u'
#S_py=S
#print(S_py[1:10])
#
#from time import time
#s=time()
#print(S.count(b'1'))
#print(time()-s)
#
#exit()
#cdef LL ans = 0
#cdef LL i
#for i in range(S.size()):
#    ans += S[i]==b'1'
#print(ans)
#print(ans//n)
#print(time()-s)
#
## python
#cdef LL nn = 10**7
#SS = readline()[:-1] * nn
#s=time()
#ans = 0
#for i in range(len(SS)):
#    ans += SS[i]==b'1'
#print(ans)
#print(ans//nn)
#
#print(time()-s)