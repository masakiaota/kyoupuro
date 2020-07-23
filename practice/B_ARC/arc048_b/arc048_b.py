# https://atcoder.jp/contests/arc048/tasks/arc048_b
# O(n**2)なら簡単
# じゃんけんがなければ簡単(ソートしてやれば良い)
# じゃんけんの手は別にcounterしておけば良い
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


# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N = a_int()
RHI = []
cnt = Counter([])

for i in range(N):
    r, h = ints()
    RHI.append((r, h, i))
    cnt[r, h] += 1
RHI.sort(key=itemgetter(0))
R, H, I = zip(*RHI)
ans = [(-1, -1, -1) for _ in range(N)]
for r, h, i in RHI:
    n_win = bisect_left(R, r)  # このrateで勝てる人数
    n_lose = N - bisect_right(R, r)  # このrateで負ける人数
    n_draw = cnt[r, h] - 1
    if h == 1:  # 自分がグーなら同点のチョキ(2)には勝つ。3には負ける
        n_win += cnt[r, 2]
        n_lose += cnt[r, 3]
    elif h == 2:
        n_win += cnt[r, 3]
        n_lose += cnt[r, 1]
    elif h == 3:
        n_win += cnt[r, 1]
        n_lose += cnt[r, 2]
    ans[i] = (n_win, n_lose, n_draw)

for a in ans:
    print(*a)
