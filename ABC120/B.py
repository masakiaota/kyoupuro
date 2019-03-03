def readln():
    return list(map(int, input().split()))


A, B, K = readln()

cnt = 0
for n in range(min(A, B), -1, -1):
    if (A % n == 0) and (B % n == 0):
        cnt += 1
    if cnt == K:
        print(n)
        break
