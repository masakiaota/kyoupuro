# https://atcoder.jp/contests/abc133/tasks/abc133_d

# Ai= xi+x(i+1) と置くと
# x1は即座に求まる。なぜならば、A1-A2+A3-A4...+AN = 2*x1となることは式変形からすぐにわかる。
# x1がわかるとx2以降は芋づる式に求まる。
# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N = int(input())
A = read_ints()

x1 = 0
for i, a in enumerate(A):
    if i & 1:  # 奇数ならば
        x1 -= a
    else:  # 偶数ならば
        x1 += a
# x1 //= 2

ans = [x1]
xi = x1
for a in A:
    xi = 2 * a - xi
    ans.append(xi)
print(*ans[:-1])
