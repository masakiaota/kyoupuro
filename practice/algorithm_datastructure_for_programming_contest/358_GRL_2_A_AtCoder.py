# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/all/GRL_2_A
# もちろんこいつもscipyで実装可能https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.csgraph.minimum_spanning_tree.html

from scipy.sparse import lil_matrix
from scipy.sparse.csgraph import minimum_spanning_tree  # この関数の引数は隣接行列

# load data
n_V, n_E = list(map(int, input().split()))
adjmat = lil_matrix((n_V, n_V))
for _ in range(n_E):
    s, t, w = list(map(int, input().split()))
    adjmat[s, t] = w
    adjmat[t, s] = w

mst = minimum_spanning_tree(adjmat)
print(int(mst.sum()))
