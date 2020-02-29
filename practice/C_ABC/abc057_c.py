# https://atcoder.jp/contests/abc057/tasks/abc057_c
# なるべく√Nに近い約数を2つ探し、それにF(A,B)を適応すればよい


def F(A, B):
    return max(len(str(A)), len(str(B)))


N = int(input())
from math import sqrt

ans = 11
for i in range(1, int(sqrt(N)) + 1):
    if N % i == 0:
        ans = min(ans, F(N // i, i))
print(ans)
