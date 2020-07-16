# https://atcoder.jp/contests/arc041/tasks/arc041_b
from itertools import product
N, M = map(int, input().split())
B = []
for _ in range(N):
    B.append(list(map(int, input())))

A = [[0] * M for _ in range(N)]
for i, j in product(range(1, N - 1), range(1, M - 1)):
    tmp = min(B[i][j + 1], B[i][j - 1], B[i + 1][j], B[i - 1][j])
    A[i][j] = tmp
    B[i - 1][j] -= tmp
    B[i + 1][j] -= tmp
    B[i][j - 1] -= tmp
    B[i][j + 1] -= tmp

for a in A:
    print(''.join(map(str, a)))
