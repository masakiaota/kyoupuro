# ダイクストラなど使うとき用の整備

# 疎行列
from scipy.sparse import csr_matrix, lil_matrix
# 変換する場合はcsrを。あとから数字を入れる場合はlilを使うと効率的

# ダイクストラ法 (正の単一始点最短経路) (全点間最短経路)
from scipy.sparse.csgraph import dijkstra
D = dijkstra(adj_mat, directed=True, indices=unko: int)
# D[i,j]でi→jに向かう最短経路
# indicesで始点を指定している場合は、D[j]でok


# ワーシャルフロイド (負を許す、全点間最短経路)()
from scipy.sparse.csgraph import floyd_warshall, NegativeCycleError

try:
    D = floyd_warshall(adj_mat)
except NegativeCycleError:
    # 負のループはだめ
    print("NEGATIVE CYCLE")
    exit()
