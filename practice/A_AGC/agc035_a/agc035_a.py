# https://atcoder.jp/contests/agc035/tasks/agc035_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce

# 成り立つ連立方程式を見ると、各項が二回ずつ出てきてることがわかる
# ということはすべての和を取ると0になるのが条件だ

N = read_a_int()
A = read_ints()
res = reduce(xor, A)
print('Yes' if res == 0 else 'No')
