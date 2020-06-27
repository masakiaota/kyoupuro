# https://atcoder.jp/contests/diverta2019/tasks/diverta2019_b
R, G, B, N = map(int, input().split())

# rR+gG+bB=Nになるようなrgbの組み合わせの個数
# N<3000なのでO(n**2)で全探索して良さそう
# r,gがgivenのとき、rR+gG-N=-bB が成り立つ整数bが存在すれば、良い
ans = 0
for r in range(0, N + 1):
    for g in range(0, N + 1):
        left = (N - (r * R + g * G))
        if left >= 0 and left % B == 0:
            ans += 1
print(ans)
