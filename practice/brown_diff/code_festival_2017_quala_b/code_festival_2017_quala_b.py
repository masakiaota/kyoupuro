# https://atcoder.jp/contests/code-festival-2017-quala/tasks/code_festival_2017_quala_b
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_ints(): return list(map(int, read().split()))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


N, M, K = read_ints()
for n, m in product(range(N + 1), range(M + 1)):
    # n行,m列塗りつぶすと？
    if K == (M - m) * n + (N - n) * m:
        print('Yes')
        exit()
print('No')
