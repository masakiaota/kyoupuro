# https://atcoder.jp/contests/abc093/tasks/arc094_a

A = list(map(int, input().split()))
A.sort()
ans = A[2] - A[1]
sa = A[2] - (A[0] + ans)

ans += sa // 2
if sa & 1:
    ans += 2
print(ans)
