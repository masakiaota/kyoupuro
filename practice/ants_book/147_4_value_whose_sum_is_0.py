from itertools import product
from bisect import bisect_left, bisect_right

n = 6
A = [-45, -41, -36, -36, 26, -32]
B = [22, -27, 53, 30, -38, -54]
C = [42, 56, -37, -75, -10, -6]
D = [-16, 30, 77, -46, 62, 45]


AB = [a + b for a, b in product(A, B)]
CD = [c + d for c, d in product(C, D)]
CD.sort()

ans = 0
for ab in AB:
    ans += bisect_right(CD, -ab) - bisect_left(CD, -ab)  # =abとなる個数
print(ans)
