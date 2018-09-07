N = int(input())
D = []
for n in range(1, N + 1):
    D.append(int(input()))

D = set(D)
print(len(D))
