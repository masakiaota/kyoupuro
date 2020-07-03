# https://atcoder.jp/contests/arc033/tasks/arc033_2
# 集合取るだけ？
import sys
read = sys.stdin.readline


def ints(): return list(map(int, read().split()))


NA, NB = ints()
A = set(ints())
B = set(ints())
print(len(A & B) / len(A | B))
