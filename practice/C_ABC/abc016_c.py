# https://atcoder.jp/contests/abc016/tasks/abc016_3
# 全点間距離を調べて、2==distを満たす点の数を調べるだけ
# ややこしいけど友達の友達のの友達はもカウントしない

from scipy.sparse.csgraph import dijkstra
from scipy.sparse import lil_matrix
N, M = list(map(int, input().split()))
adj_mat = lil_matrix((N, N))
for _ in range(M):
    a, b = list(map(int, input().split()))
    adj_mat[a - 1, b - 1] = 1
    adj_mat[b - 1, a - 1] = 1

D = dijkstra(adj_mat)

for i in range(N):
    print((D[i] == 2).sum())
