N = int(input())
A = list(map(int, input().split()))

mean = sum(A) / N
# print(mean)
err = abs(A[0]-mean)
ans = 0
for i, a in enumerate(A):
    # print(i, err)
    if abs(a - mean) < err:
        # print(i, err)
        err = abs(a - mean)
        ans = i

print(ans)
