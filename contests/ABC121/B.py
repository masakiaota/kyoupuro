def readln():
    return list(map(int, input().split()))


N, M, C = readln()
B = readln()

A = [readln() for _ in range(N)]

ans = 0
for a in A:
    tmp = sum([aa * b for aa, b in zip(a, B)]) + C
    if tmp > 0:
        ans += 1

print(ans)
