# https://atcoder.jp/contests/abc051/tasks/abc051_d
# 考えたこと
# 1.経路復元
# 2.ダイクストラ時に使った辺記録(こっちで解いてみる)


# あとで必要なのでクラスを準備しておく
from heapq import heapify, heappop, heappush, heappushpop

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


class PriorityQueue:
    def __init__(self, heap):
        '''
        heap ... list
        '''
        self.heap = heap
        heapify(self.heap)

    def push(self, item):
        heappush(self.heap, item)

    def pop(self):
        return heappop(self.heap)

    def pushpop(self, item):
        return heappushpop(self.heap, item)

    def __call__(self):
        return self.heap

    def __len__(self):
        return len(self.heap)


INF = 10**9 + 1


def dijkstra(adj: list, s: int, N: int):
    Color = [0] * N  # 0:未訪問 1:訪問経験あり 2:訪問済み(そこまでの最短経路は確定済み)
    D = [INF] * N  # 本のdと同じ 始点からの距離を記録する
    P = [None] * N  # 本のpと同じ 最短経路木における親を記録する

    # スタートとするノードについて初期化
    D[s] = 0  # スタートまでの距離は必ず0
    P[s] = None  # 親がない(ROOTである)ことを示す
    PQ = PriorityQueue([(0, s)])  # (コスト, ノード番号)で格納 こうするとPQでソートするときに扱いやすい

    # while True:
    while PQ:  # PQに候補がある限り続ける
        # min_cost = INF
        # for i in range(N):
        #     if Color[i] != 2 and D[i] < min_cost:
        #         min_cost = D[i]
        #         u = i
        min_cost, u = PQ.pop()  # 上記の処理はこのように簡略化できる

        Color[u] = 2  # uは訪問済み これから最短経路木に突っ込む作業をする

        if D[u] < min_cost:
            # もし今扱っているmin_costがメモしているやつよりも大きいなら何もしないで次へ(メモしている方を扱えばいいので)
            continue

        # 以下のforではsからの最短経路木にuを追加したときの更新、uまわりで次の最短経路になる候補の更新をしている
        for idx_adj_u, w_adj_u in adj[u]:
            if Color[idx_adj_u] != 2:
                # 訪問済みでなく、u→vへの経路が存在するならば
                if D[u] + w_adj_u < D[idx_adj_u]:
                    D[idx_adj_u] = D[u] + w_adj_u  # u周りにおける最短経路の候補の更新
                    P[idx_adj_u] = u
                    PQ.push((D[idx_adj_u], idx_adj_u))  # ここで候補に追加していっている
                    Color[idx_adj_u] = 1

    return D, P


# load datas
N, M = read_ints()
adj = [[] for _ in range(N)]  # adj[node_id]=(隣接nodei,コスト)で格納する
for _ in range(M):
    a, b, c = read_ints()
    a -= 1
    b -= 1
    adj[a].append((b, c))
    adj[b].append((a, c))

used_edges = set()
for s in range(N):
    _, P = dijkstra(adj, s, N)
    for i, p in enu(P):
        if p == None:
            continue
        if p > i:
            i, p = p, i
        used_edges.add((i, p))

print(M - len(used_edges))
