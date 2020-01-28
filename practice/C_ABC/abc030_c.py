# https://atcoder.jp/contests/abc030/tasks/abc030_c

# Aから出発する時間とBから出発する時間を並べていって、greedyに乗っていく

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
X, Y = read_ints()
A = read_ints()
B = read_ints()
A.extend(B)

itenerary = []
for i, a in enumerate(A):
    if i < N:
        fro = 0  # 空港Aから出発
    else:
        fro = 1  # 空港Bから出発
    itenerary.append((a, fro))

itenerary.sort()

now = 0
fro = 0
cnt = 0
for t, p in itenerary:
    if now > t:
        continue
    if fro == p:
        cnt += 1
        if fro == 0:
            now = t + X
        else:
            now = t + Y
        fro = 1 - fro

print(cnt // 2)
