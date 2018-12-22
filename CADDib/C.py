N, P = list(map(int, input().split()))
if N == 1:
    print(P)
    exit()
elif P == 1:
    print("1")
    exit()

ans = 1
tmp = P

for i in range(2, int(pow(P, 1 / N)) + 1):
    po = pow(i, N)
    while tmp % po == 0:
        tmp = tmp // po
        ans = ans * i
        # print(i, tmp, ans)

    if tmp < po:
        break

print(ans)

# 攻略の鍵はint(pow(P, 1 / N)) + 1でforに制限をつけるところでしたね
