# https://atcoder.jp/contests/abc089/tasks/abc089_c
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from itertools import product, permutations, combinations

N = read_a_int()
cnt = defaultdict(lambda: 0)
march = {'M', 'A', 'R', 'C', 'H'}
for _ in range(N):
    s = read()[0]
    if s in march:
        cnt[s] += 1
ans = 0
for a, b, c in combinations(cnt.values(), r=3):
    ans += a * b * c
print(ans)
