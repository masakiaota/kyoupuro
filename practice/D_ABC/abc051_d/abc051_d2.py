# https://atcoder.jp/contests/abc051/tasks/abc051_d
# 想定解法
# 最短距離の隣接行列Dが与えられたときに、edges[a, b]が最短経路に含まれるかどうか考える
# 2点s,tについて、最短距離であれば、D[s,t]=D[s,i]+edges[i,j]+D[j,t]を満たすi,jの組み合わせは最短経路辺である
# 5000*1000の探索なので間に合う

from scipy.sparse import csr_matrix  # 自分で配列を作ってからcsrに入れよう(lilに打ち込んでいくのは非常に遅い)

# ダイクストラ法 (正の単一始点最短経路) (全点間最短経路)
from scipy.sparse.csgraph import dijkstra
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
adj_mat = [[0] * N for _ in range(N)]
INF = 10 ** 9
from collections import defaultdict
from itertools import combinations
edges = defaultdict(lambda: INF)
for _ in range(M):
    a, b, c = read_ints()
    a -= 1
    b -= 1
    adj_mat[a][b] = c
    adj_mat[b][a] = c
    edges[a, b] = c

adj_mat = csr_matrix(adj_mat, dtype='int')
D = dijkstra(adj_mat, directed=False)
# print(D)

# 最短経路じゃない辺全探索
edge_not_use = set()
# 辺を固定してs,tを全探索したほうが良さそう
# だけどこれはTLE(pythonが遅いせい)
# for (i, j), e in edges.items():
#     for s, t in combinations(range(N), 2):
#         d = D[s, t]
#         if d == D[s, i] + e + D[j, t] or d == D[t, i] + e + D[j, s]:
#             break
#     else:
#         edge_not_use.add((i, j))

# よく考えてみれば
# D[s,j]=D[s,i]+edges[i,j]のedges[i,j]は最短経路の一部になっているはず(s→jの最短経路の一部を成す)
# sを全探索するだけでいい

for (i, j), e in edges.items():
    for s in range(N):
        d = D[s, j]
        if d == D[s, i] + e or d == D[s, j] + e:
            break
    else:
        edge_not_use.add((i, j))


print(len(edge_not_use))
# print(edge_not_use)
