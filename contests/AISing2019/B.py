N = int(input())
A, B = list(map(int, input().split()))
P = list(map(int, input().split()))

P.sort()

c1, c2, c3 = 0, 0, 0
for i, p in enumerate(P, 1):
    if p <= A:
        c1 = i
    if (p > A) and (p <= B):
        c2 = i
    if p > B:
        c3 = i


if c1 <= c2 <= c3:
    print(min((c1), (c2 - c1), (c3 - c2)))
else:
    print(0)
