# 木上の累積和


cimport cython

import sys
from collections import defaultdict
sys.setrecursionlimit(1 << 24)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))


cdef ints():
    return list(map(int, read().split()))


cdef:
    long long N, Q, a, b, q, x, _
    long cnt[200005]


N, Q = ints()
tree = [[] for _ in range(N)]
for _ in range(N - 1):
    a, b = mina(*ints())
    tree[a].append(b)
    tree[b].append(a)

for _ in ra(Q):
    q, x = ints()
    cnt[q - 1] += x
# dfsでcntに木に沿った累積和をsetしていく


cdef dfs(int u, int p):  # uは現在のノード、pは親のノード
    cdef long nv
    cnt[u] += cnt[p]
    for nv in tree[u]:
        if nv == p:
            continue
        dfs(nv, u)

dfs(0, N + 1)

for i in range(N):
    print(cnt[i], end=' ')
