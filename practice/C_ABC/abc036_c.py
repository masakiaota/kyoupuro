# https://atcoder.jp/contests/abc036/tasks/abc036_c
# 下からreplaceするための番号を降っていけばいい気がするけど


N = int(input())
A = []
for _ in range(N):
    A.append(int(input()))

A_unique = list(set(A))
A_unique.sort()
replace = {}
cnt = 0
for key in A_unique:
    replace[key] = cnt
    cnt += 1

for a in A:
    print(replace[a])
