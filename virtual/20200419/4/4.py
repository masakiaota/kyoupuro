# 木上の累積和

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


from collections import defaultdict

N, Q = read_ints()
tree = defaultdict(lambda: [])
for _ in range(N - 1):
    a, b = read_ints()
    a -= 1
    b -= 1
    tree[a].append(b)
    tree[b].append(a)
cnt = [0] * N
for _ in ra(Q):
    q, x = read_ints()
    cnt[q - 1] += x

# dfsでcntに木に沿った累積和をsetしていく


def dfs(c, u, p):  # uは現在のノード、pは親のノード、cはuは含まない累積のスコア
    cnt[u] += c
    for nv in tree[u]:
        if nv == p:
            continue
        dfs(cnt[u], nv, u)


dfs(0, 0, -1)

print(*cnt)
