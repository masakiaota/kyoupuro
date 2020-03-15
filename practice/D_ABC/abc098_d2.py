# https://atcoder.jp/contests/abc021/tasks/abc021_c
# 面白い #幅優先探索せずにもっと楽に実装

# 街cについて、最短距離dist(a,b)=dist(a,c)+dist(c,b)を満たすcが高橋くんの歩いた候補である。
# 上記を満たし且つ、(dist(a,c),dist(c,b))のパターンについてカウントすると、
# 各距離における場合の数が求まるのであとは累積すれば良い

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


from collections import defaultdict
from scipy.sparse import csr_matrix, lil_matrix
# 変換する場合はcsrを。あとから数字を入れる場合はlilを使うと効率的
# ダイクストラ法 (正の単一始点最短経路) (全点間最短経路)
from scipy.sparse.csgraph import dijkstra
MOD = 10**9 + 7
N = read_a_int()
a, b = read_ints()
a -= 1
b -= 1
M = read_a_int()
adj_mat = lil_matrix((N, N), dtype='int')
for _ in range(M):
    x, y = read_ints()
    x -= 1
    y -= 1
    adj_mat[x, y] = 1
    adj_mat[y, x] = 1


D1 = dijkstra(adj_mat, directed=False, indices=b)
D2 = dijkstra(adj_mat, directed=False, indices=a)
d = D1[a]
# dist(a,b)=dist(a,c)+dist(c,b)みたした場合に、
# (dist(a,c),dist(c,b))のcntを取っておく

cnt = defaultdict(lambda: 0)
for d1, d2 in zip(D1, D2):
    if d1 + d2 == d:
        cnt[(d1, d2)] += 1
ans = 1
for v in cnt.values():
    ans *= v
    if ans >= MOD:
        ans %= MOD

print(ans)
