N = int(input())
A = list(map(int, input().split()))
import sys

sys.setrecursionlimit(100000000)


def gcd(a, b):
    if b == 0:
        return a
    return gcd(b, a % b)


ans = A[0]
for a in A[1:]:
    ans = gcd(ans, a)

print(ans)
