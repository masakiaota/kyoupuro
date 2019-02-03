N, M = list(map(int, input().split()))
X = list(map(int, input().split()))

if M <= N:
    print(0)
    exit()

X.sort(reverse=True)

diff = []
for x_l, x_s in zip(X[:-1], X[1:]):
    diff.append(x_l - x_s)

diff.sort(reverse=True)

print(sum(diff[N-1:]))
