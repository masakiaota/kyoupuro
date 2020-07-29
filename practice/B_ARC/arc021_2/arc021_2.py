# https://atcoder.jp/contests/arc021/tasks/arc021_2
# 連立方程式ごちゃごちゃやってたらなんか解けた

from functools import reduce
from operator import xor
L, *B = map(int, open(0).read().split())
if reduce(xor, B) != 0:
    print(-1)
    exit()
A = [0] * L
for i in range(L - 1):
    A[i + 1] = B[i] ^ A[i]
print(*A, sep='\n')
