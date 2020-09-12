# 読み込みはpython側で行う
import sys
readline = sys.stdin.buffer.readline  # byte
read = sys.stdin.readline  # 文字列読み込む時はこっち


from time import time
# python
nn = 10**7
SS = readline()[:-1] * nn
s = time()
anss = 0
for ss in SS:
    anss += ss == b'1'
print(anss)
print(anss // nn)

print(time() - s)
