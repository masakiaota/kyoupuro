

import sys
read = sys.stdin.readline
ra = range
enu = enumerate
MOD = 10**9 + 7


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
A = read_ints()
B = []
for _ in ra(K):
    B.extend(A)
# 愚直に転倒数を求める
n_inv = 0
for i in ra(len(B)):
    for j in ra(i + 1, len(B)):
        if B[i] > B[j]:
            n_inv += 1
print(n_inv % MOD)
