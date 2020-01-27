# https://atcoder.jp/contests/abc028/tasks/abc028_d
# kが中央値になる確率を求める問題

# 三回試行してkが中央値になる状況は以下に分けて考えられる
# 三回ともx==kとなる (1通り)
# 二回はx==kとなって、一回はx!=kとなる (3通り)
# x1<x2<x3 としたとき、x2==kとなる (6通り)

# 以上の議論により
# p(kが中央値) = p(k==x)**3 + 3*(p(k==x)**2)*p(k!=x) + 6*p(x<k)*p(x>k)*p(x==k)
# である
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
p_eq = 1 / N
p_neq = 1 - p_eq
p_lt = (K - 1) / N
p_gt = (N - K) / N
print(p_eq**3 + 3 * (p_eq**2) * p_neq + 6 * p_lt * p_gt * p_eq)
