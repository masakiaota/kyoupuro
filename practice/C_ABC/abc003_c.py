# https://atcoder.jp/contests/abc003/tasks/abc003_3
# easy
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
R = read_ints()
R.sort()
ans = 0
for r in R[-K:]:
    ans = (ans + r) / 2
print(ans)
