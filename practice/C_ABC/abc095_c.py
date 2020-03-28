# https://atcoder.jp/contests/abc095/tasks/arc096_a


# A1枚→A円
# B1枚→B円
# A,B両方一枚→AB円


# もし2AB<=A+Bならmin(X,Y)になるまで買ったほうが安い
# Aだけ必要→A<AB→Aを買う、違うならABを買う
# Bだけ必要→上に同じ

# もし2AB<=A+Bでなかった場合
# A,B<ABの場合→A,Bを個別に買う
# A<2AB の場合→Aは個別で
# この議論は上に統合される

a, b, c, x, y = map(int, input().split())
# 両方買うほうがお得のときは先に買っておく
ans = 0
if 2 * c <= a + b:
    n = min(x, y)
    ans += 2 * n * c
    x -= n
    y -= n

# print(x, y, ans)

# Aだけ買う
if a <= 2 * c:
    ans += x * a
else:
    ans += x * 2 * c

if b <= 2 * c:
    ans += y * b
else:
    ans += y * 2 * c
print(ans)
