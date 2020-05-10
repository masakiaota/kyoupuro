# http://poj.org/problem?id=3723
# scipyを使って楽に実装できる
from scipy.sparse.csgraph import minimum_spanning_tree  # この関数の引数は隣接行列
from scipy.sparse import csr_matrix


N = 5
M = 5
R = 8

XYD = [(4, 3, 6831),
       (1, 3, 4583),
       (0, 0, 6592),
       (0, 1, 3063),
       (3, 3, 4975),
       #    (1, 3, 2049), #二重辺の処理がめんどくさいので ここで消しておく
       (4, 2, 2104),
       (2, 2, 781), ]

# 頭良すぎか？
# 親密度を負のコストだと見立てれば、最小全域木の順番に徴兵するのが一番コストのかからない順番になる。(始めるノードによらない)

edges = []
n_nodes = N + M  # 前N個のノードを男性用にする
row = []
col = []
cost = []
for x, y, d in XYD:
    row.append(x)
    col.append(N + y)
    cost.append(-d)
    # col.append(x)
    # row.append(N + y)
    # cost.append(-d)
adj_mat = csr_matrix((cost, (row, col)),
                     shape=(n_nodes, n_nodes), dtype='int64')

mst = minimum_spanning_tree(adj_mat)
print(mst.sum())
print(mst)
print(int(10000 * n_nodes + mst.sum()))
