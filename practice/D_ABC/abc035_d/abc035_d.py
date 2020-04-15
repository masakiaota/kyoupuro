# https://atcoder.jp/contests/abc035/tasks/abc035_d

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter

# 有向グラフ
# 0→0の閉路の集合Dに対して、
# max_{d \in D} (max_{u \in d}A_u * (T-sum_{辺{u→v} \in d}(c_{u→v}))) が求まれば良い
# 問題は閉路のパターンが多いときに計算量が多くなること。 # dp的な発想で→A次第で今まで確定した状態がひっくり返るのでだめ
# A_u * (T-(uを含む閉路の中の最短距離))なのだから、0→uの最短経路さえわかれば難しくない！

# Dijkstraで0→u for any uの最短経路計算
# d(0,u)+d(u,0)←これ計算できなくね？
# いや、方向をひっくり返してダイクストラすればいいのでは?(これだ)

N, M, T = read_ints()
A = read_ints()


def read_graph(N: int, directed=True):
    '''
    graphを読み込んだcsr_matrixを返す Nはノード数
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
        if not directed:
            fr.append(b)
            to.append(a)
            co.append(c)
    return csr_matrix((co, (fr, to)), shape=(N, N), dtype='int64')


from scipy.sparse.csgraph import dijkstra
adj_mat = read_graph(N)
D = dijkstra(adj_mat, directed=True, indices=0)
D_inv = dijkstra(adj_mat.T, directed=True, indices=0)
ans = -1
for u in range(N):
    money = (T - (D[u] + D_inv[u])) * A[u]
    ans = max(money, ans)
print(int(ans))
