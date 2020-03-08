# https://atcoder.jp/contests/abc074/tasks/arc083_a
# Cだけど水パフォ

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))

# 水と砂糖の操作は独立にできる
# 知りたいのは砂糖の質量と水の質量
# 砂糖の質量の増加の仕方と水の質量の増加の仕方は全探索できる
# F<3000なので、1グラムずつ調べても10*7でpypyならギリok


from itertools import product
A, B, C, D, E, F = read_ints()

# 水の質量の増加の仕方
# Aの倍数とBの倍数はありえる #かつF[g]以下
candi_water = set()
for pa, pb in product(range(F // (A ** 100) + 2), range(F // (B ** 100) + 2)):
    candi = pa * A * 100 + pb * B * 100
    if candi > F:
        continue
    candi_water.add(candi)


# 砂糖の質量の増加の仕方
candi_suger = set()
for pa, pb in product(range(F // (C) + 2), range(F // (D) + 2)):
    candi = pa * C + pb * D
    if candi > F:
        continue
    candi_suger.add(candi)


# 全組み合わせを全探索
most = -1
ans = (-1, -1)
# print(candi_water, candi_suger)
for a, b in product(candi_water, candi_suger):
    if a == 0 or a + b > F or 100 * b > E * a:  # ビーカーに入らないか溶け残るか
        continue
    noudo = b / (a + b)
    if noudo > most:
        most = noudo
        ans = (a + b, b)
        # print(most, ans)
print(*ans)
