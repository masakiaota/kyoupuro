# 木上の累積和

import sys
from collections import defaultdict
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))


def ints():
    return list(map(int, read().split()))


N, Q = ints()
tree = defaultdict(lambda: [])
for _ in range(N - 1):
    a, b = mina(*ints())
    tree[a].append(b)
    tree[b].append(a)
cnt = [0] * N
for _ in ra(Q):
    q, x = ints()
    cnt[q - 1] += x
cnt.append(0)  # -1アクセス用

# dfsでcntに木に沿った累積和をsetしていく


def dfs(u, p):  # uは現在のノード、pは親のノード
    cnt[u] += cnt[p]
    for nv in tree[u]:
        if nv == p:
            continue
        dfs(nv, u)


dfs(0, -1)

print(*cnt[:-1])
