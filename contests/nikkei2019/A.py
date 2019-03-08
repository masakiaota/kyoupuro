N, A, B = list(map(int, input().split()))

ans = (A+B)-N if A+B > N else 0

print(min(A, B), ans)
