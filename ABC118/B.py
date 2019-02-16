# import numpy as np
N, M = list(map(int, input().split()))

A = [list(map(int, input().split()))[1:] for _ in range(N)]

cnt = {}
for i in range(1, M+1):
    cnt[i] = 0

for A_row in A:
    for a in A_row:
        cnt[a] += 1
ans = 0
for i in range(1, M+1):
    if N == cnt[i]:
        ans += 1
print(ans)
