N = int(input())
X = [tuple(input().split()) for _ in range(N)]
ans = 0
for x, u in X:
    x = float(x)
    if u == 'BTC':
        x *= 380000
    ans += x

print(ans)
