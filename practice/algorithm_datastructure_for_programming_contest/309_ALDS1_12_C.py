# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/12/ALDS1_12_C
# この問題では隣接行列を使うとメモリ不足、priority queueを用いないと時間超過という結果になる
# 全問の隣接リスト形式に加えてpriority queueを適応することでダイクストラ法を更に高速化する
# u周辺のコストの最小値を探すところでpriority queueを使うことで最小値探索をO(n)からO(logn)まで減らす
# Dへのメモもしておくけど最小値を取り出すのはPQでやりたいというお気持ち


INF = 10**6 + 1
# あとで必要なのでクラスを準備しておく
from heapq import heapify, heappop, heappush, heappushpop


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


def dijkstra(adj: list, s: int):
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
N = int(input())
adj = [[] for _ in range(N)]  # adj[node_id]=(隣接nodei,コスト)で格納する
'''
例
[
(0について) [(1,1235), (4,65)]
(1について) [(20,6000)]
...
]
'''
for _ in range(N):
    tmp = list(map(int, input().split()))
    if tmp[1] != 0:
        node = tmp[0]
        for i in range(2, 2 + tmp[1] * 2, 2):
            adj[node].append((tmp[i], tmp[i + 1]))

D, P = dijkstra(adj, 0)

for i in range(N):
    print(i, D[i])
