H, W = list(map(int, input().split()))

A = []
for h in range(H):
    A.append(str(input()))

print("#" * (W + 2))
for a in A:
    print("#" + a + "#")
print("#" * (W + 2))
