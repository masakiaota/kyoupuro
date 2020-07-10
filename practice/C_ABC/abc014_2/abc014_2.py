n, X = map(int, input().split())
A = list(map(int, input().split()))
ans = 0
for i in range(n):
    ans += A[i] if (X >> i) & 1 else 0
print(ans)
