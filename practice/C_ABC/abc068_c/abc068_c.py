# https://atcoder.jp/contests/abc068/tasks/arc079_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


from collections import defaultdict
N, M = ints()
# グラフを作ってたかだか2回でNに行けるかを探索する(深さ優先が楽かな？)
graph = defaultdict(lambda: [])
for _ in ra(M):
    a, b = mina(*ints())
    graph[a].append(b)
    graph[b].append(a)


def dfs(now, fr, d):
    if d == 2:
        return now == N - 1
    res = False
    for to in graph[now]:
        if fr == to:
            continue
        res = res or dfs(to, now, d + 1)
    return res


print('POSSIBLE' if dfs(0, -1, 0) else 'IMPOSSIBLE')
