# https://atcoder.jp/contests/abc077/tasks/arc084_a
# bisectで行けるけどめぐる式の練習

# Bを固定すると、A,Cを二分探索すれば条件を満たす個数が計算できる

from bisect import bisect_left, bisect_right
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
B = read_ints()
C = read_ints()

A.sort()
C.sort()

ans = 0
for b in B:
    ans += bisect_left(A, b) * (N - bisect_right(C, b))
    # Cに関しては真に大きい個数を知りたいことに注意
print(ans)
