input()
A = list(map(int, input().split()))
A = sorted(A)[::-1]

alice, bob = 0, 0
for i, a in enumerate(A):
    if i % 2 == 0:
        alice += a
    else:
        bob += a

print(alice - bob)
