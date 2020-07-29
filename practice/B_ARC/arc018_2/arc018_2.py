# https://atcoder.jp/contests/arc018/tasks/arc018_2
# 普通に全探索じゃないの！？
# 面積が0.5*10^18ぐらいになるので小数点だとしぬ

from itertools import combinations
N = int(input())
P = [complex(*map(int, input().split())) for _ in range(N)]
ans = 0
for i, j, k in combinations(range(N), r=3):
    a, b = P[j] - P[i], P[k] - P[i]
    S = abs(int(a.real) * int(b.imag) - int(a.imag) * int(b.real))   # 平行四辺形の面積
    ans += (~S & 1) and (S != 0)
print(ans)
