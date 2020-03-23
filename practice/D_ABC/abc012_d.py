# https://atcoder.jp/contests/abc012/tasks/abc012_4
# arg_D[i,j] min_i max_j D[i,j] を解く。Dはi→jへの最短距離


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


from scipy.sparse import csr_matrix
from scipy.sparse.csgraph import dijkstra

N, M = read_ints()
# scipy.csgraphを使う→隣接行列(疎行列)
adj_mat = [[0] * N for _ in range(N)]
for _ in range(M):
    a, b, t = read_ints()
    a -= 1
    b -= 1
    adj_mat[a][b] = t  # a→bまでの距離
    adj_mat[b][a] = t
adj_mat = csr_matrix(adj_mat, dtype='int')  # 隣接行列完成
D = dijkstra(adj_mat, directed=False)  # 全点間 (N,N) i→jまでの最短距離

print(int(D.max(axis=0).min()))
