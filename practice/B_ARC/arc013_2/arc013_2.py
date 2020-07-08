# https://atcoder.jp/contests/arc013/tasks/arc013_2
C = int(input())
a = b = c = 0
for _ in range(C):
    NML = list(map(int, input().split()))
    n, m, l = sorted(NML)
    a = max(a, n)
    b = max(b, m)
    c = max(c, l)
print(a * b * c)
