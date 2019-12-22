# https://atcoder.jp/contests/abc127/tasks/abc127_c
N, M = list(map(int, input().split()))
L = 0
R = N
for _ in range(M):
    l, r = list(map(int, input().split()))
    L = max(L, l)
    R = min(R, r)

print(max(R - L + 1, 0))
