import sys
read = sys.stdin.readline


def readln():
    return list(map(int, read().split()))


N, M = readln()

A = []
B = []
data = {}
for _ in range(N):
    a, b = readln()
    A.append(a)
    B.append(b)
    data[a] = 0

for a, b in zip(A, B):
    data[a] += b

ans = 0
drink = 0
for k, v in sorted(data.items()):
    ans += k * v
    drink += v
    if drink > M:
        break
print(ans-k*(drink-M))
