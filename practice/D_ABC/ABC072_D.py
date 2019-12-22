N = int(input())
P = list(map(int, input().split()))

ans = 0
# print(P)

for i in range(N - 1):
    if P[i] != int(i + 1):
        continue
    else:
        P[i], P[i + 1] = P[i + 1], P[i]
        # print(i + 1, P)
        ans += 1


# 最後を見てなかった！！！！
if P[-1] == N:
    ans += 1

print(ans)
