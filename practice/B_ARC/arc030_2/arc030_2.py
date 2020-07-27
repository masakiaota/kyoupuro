# https://atcoder.jp/contests/arc030/tasks/arc030_2
# 普通にdfsを工夫するだけじゃない？
# そのパスの中で一番外と訪問済みの枝までの距離だけ計上したい
# →一番外側を葉としたdfsをすればよい
# →余分な枝を切り落とせば良い
# 始点と宝石をつなぐ経路は必ず必要
# O(n^2)かけて愚直に宝石までの経路を列挙するか？

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


def ints(): return list(map(int, read().split()))


# default import
from collections import defaultdict
n, x = ints()
x -= 1
H = ints()
graph = defaultdict(lambda: [])
for _ in range(n - 1):
    a, b = mina(*ints())
    graph[a].append(b)
    graph[b].append(a)

candi = set()
path = []  # そのノードにたどり着くためのpath(自身も含む)


def dfs(u, p):
    # 入るときの処理
    path.append(u)
    if H[u] == 1:  # 宝石があればそれまでの経路を使用
        candi.update(path)
    for nx in graph[u]:
        if nx == p:
            continue
        dfs(nx, u)
    # 抜けるときの処理
    path.pop()


dfs(x, -1)
print(max(0, (len(candi) - 1) * 2))
