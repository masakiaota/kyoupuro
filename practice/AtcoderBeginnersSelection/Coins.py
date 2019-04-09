A = int(input())
B = int(input())
C = int(input())
X = int(input())

x = int(X / 50)
cnt = 0
for a in range(A + 1):
    for b in range(B + 1):
        for c in range(C + 1):
            if (10 * a + 2 * b + c) == x:
                cnt += 1

print(cnt)
