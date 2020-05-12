# https://atcoder.jp/contests/abc148/tasks/abc148_f


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def dist_on_tree(tree, s, N):
    '''tree...(to,cost)の隣接リスト形式
    s...始点ノード
    N...ノード数'''
    from collections import deque
    D = [-1] * N
    D[s] = 0
    que = deque([(s, 0)])  # 現在のノード、そこまでの距離
    while que:
        now, dist = que.popleft()
        for to, cost in tree[now]:
            if D[to] != -1:
                continue
            D[to] = dist + cost
            que.append((to, dist + cost))
    return D


INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations

N, u, v = read_ints()
tree = defaultdict(lambda: [])
for _ in range(N - 1):
    a, b = read_ints()
    a -= 1
    b -= 1
    tree[a].append((b, 1))
    tree[b].append((a, 1))


# u...逃げる v...追いかける
# u,vからxへの距離をud[x],vd[x]とすると、ud[x]<vd[x]を満たすxが答えの候補となる(∵ud[x]>=vd[x]は追いかける方の陣地、その中に突っ込むのは必ず不利 =は距離が同じ、ということはその前に衝突している
# ud[x]<=vd[x]のなかで最も大きなvd[x]-1が答え(uが壁から跳ね返ってvが動かずとも衝突する)

ud = dist_on_tree(tree, u - 1, N)
vd = dist_on_tree(tree, v - 1, N)
dist = ud[v - 1]
# print(ud)
# print(vd)

ans = 0
for x in range(N):
    if ud[x] >= vd[x]:
        continue
    ans = max(ans, vd[x] - 1)

print(ans)
