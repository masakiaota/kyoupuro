# ダイクストラなど使うとき用の整備

# 疎行列
from scipy.sparse import csr_matrix  # 自分で配列を作ってからcsrに入れよう(lilに打ち込んでいくのは非常に遅い)


def read_graph(N: int, M: int, directed=True):
    '''
    graphを読み込んだcsr_matrixを返す Nはノード数 Mは読み込み行数
    '''
    from scipy.sparse import csr_matrix
    fr, to, co = [], [], []  # from, to, cost
    for _ in range(M):
        a, b, c = read_ints()
        a -= 1
        b -= 1
        fr.append(a)
        to.append(b)
        co.append(c)
        if directed == False:
            fr.append(b)
            to.append(a)
            co.append(c)
    # 二重辺がある場合はcoの値が足されてしまうので注意
    return csr_matrix((co, (fr, to)), shape=(N, N), dtype='int64')


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


# 最短距離行列Dが得られたときにs→tまでの経路を復元する
def restore_path(s: int, t: int, edges: dict, D: list):
    '''
    s→tまでの最短経路を復元する
    edgesはオリジナルの辺のリスト edges[a,b]=a,bを結ぶ辺の長さ。ないときは(INF、自分自身へもINF)
    Dは最短経路D[s][t]=s→tの最短経路
    '''
    ret = []  # 経路
    cur = s
    while cur != t:
        for i in range(N):  # Nはノード数 = len(D) or D.shape[0]
            if i != cur and edges[cur, i] + D[i][t] == D[cur][t]:
                ret.append((cur, i) if cur < i else (i, cur))  # 大小関係をはっきりさせておく
                cur = i
                break
    return ret


# 全域最小木 クラスカル法
from scipy.sparse.csgraph import minimum_spanning_tree
mst = minimum_spanning_tree(adj_mat)
