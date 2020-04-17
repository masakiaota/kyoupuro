# https://atcoder.jp/contests/abc101/tasks/arc099_a


import sys
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
if N == K:
    print(1)
    exit()
A = read_ints()

K -= 1
i = A.index(1)

# 怒りの全探索
ans = 0
for j in range(0, N - 1, K):
    ans += 1

print(ans)
