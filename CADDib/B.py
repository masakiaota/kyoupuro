N, H, W = list(map(int, input().split()))

sizels = []
for _ in range(N):
    sizels.append(list(map(int, input().split())))

cnt = 0
for a, b in sizels:
    if (a >= H) and (b >= W):
        # print(a, b, "OK")
        cnt += 1

print(cnt)
