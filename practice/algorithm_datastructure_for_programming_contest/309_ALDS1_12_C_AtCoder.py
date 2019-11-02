# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/12/ALDS1_12_C
# これに関してもAtCoderではすでにある実装が使える https://note.nkmk.me/python-scipy-shortest-path/
# じゅっぴーさんの記事 https://juppy.hatenablog.com/entry/2019/06/04/scipy%E3%81%AEFloyd-Warshall%E3%81%A8Dijkstra%E3%81%AE%E3%81%99%E3%81%99%E3%82%81_Python_%E7%AB%B6%E6%8A%80%E3%83%97%E3%83%AD%E3%82%B0%E3%83%A9%E3%83%9F%E3%83%B3%E3%82%B0_Atcoder_1
# 1つ目の記事にあるようにdijkstraでなくshortest_path関数に引数を入れるのが実用的な使い方か

INF = 10**6

from scipy.sparse.csgraph import dijkstra
from scipy.sparse import csr_matrix, lil_matrix
# scipy.sparse.csgraphでは基本的に隣接行列の入力を想定している
# 機械学習ではcsrが基本的だがlil_matrixのほうがデータを打ち込むのが早いらしい


# load datas
N = int(input())
# adj_mat = csr_matrix((N, N))
adj_mat = lil_matrix((N, N))
# print(adj_mat.shape)
# print(adj_mat)

for _ in range(N):
    tmp = list(map(int, input().split()))
    if tmp[1] != 0:
        node = tmp[0]
        for i in range(2, 2 + tmp[1] * 2, 2):
            adj_mat[node, tmp[i]] = tmp[i + 1]

D = dijkstra(adj_mat)[0]
# 行ごとにその行を始点としたときの各ノードへの最短経路が計算されるのでそれを取り出すだけ

for i in range(N):
    print(i, int(D[i]))
