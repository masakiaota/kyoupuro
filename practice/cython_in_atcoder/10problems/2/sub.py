mycode = r'''
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
'''

import sys
if sys.argv[-1] == 'ONLINE_JUDGE':  # コンパイル時
    import os
    with open('mycode.pyx', 'w') as f:
        f.write(mycode)
    os.system('cythonize -i -3 -b mycode.pyx')

import mycode
