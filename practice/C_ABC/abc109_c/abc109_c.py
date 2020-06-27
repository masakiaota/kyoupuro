# https://atcoder.jp/contests/abc109/tasks/abc109_c
# Xを中心にしたときの最大公約数かな
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


from functools import reduce
from math import gcd

N, zero = ints()
X = [abs(x - zero) for x in ints()]
# if N == 1:
#     exit(X[0] - zero)
print(reduce(gcd, X))
