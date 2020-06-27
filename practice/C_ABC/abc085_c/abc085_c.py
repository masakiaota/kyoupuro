# https://atcoder.jp/contests/abc085/tasks/abc085_c
# 10000a+5000b+1000c=Yが成り立つa,b,cでa+b+c=Nとなる組み合わせが存在するか？

# O(N**2)で全探索すれば良い
# c=(Y-(10000a+5000b))//1000

N, Y = map(int, input().split())

for a in range(N + 1):
    if Y - a * 10000 < 0:
        break
    for b in range(N + 1):
        c = (Y - 10000 * a - 5000 * b) // 1000
        if c < 0:
            break
        if a + b + c == N:
            print(a, b, c)
            exit()
print(-1, -1, -1)
