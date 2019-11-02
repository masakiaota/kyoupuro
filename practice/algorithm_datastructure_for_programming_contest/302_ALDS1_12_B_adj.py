# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/12/ALDS1_12_B
# この問題の隣接行列を用いたver

INF = 10**6 + 1

# load datas
N = int(input())
adj = [[] for _ in range(N)]  # (node_id, 隣接要素, 隣接nodeidとコスト)の順で格納する
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

# from pprint import pprint
# pprint(adj)

# これで隣接リストはできた


def dijkstra(adj: list, s: int):
    Color = [0] * N  # 0:未訪問 1:訪問経験あり 2:訪問済み(そこまでの最短経路は確定済み)
    D = [INF] * N  # 本のdと同じ 始点からの距離を記録する
    P = [None] * N  # 本のpと同じ 最短経路木における親を記録する

    # スタートとするノードについて初期化
    D[s] = 0  # スタートまでの距離は必ず0
    P[s] = None  # 親がない(ROOTである)ことを示す

    while True:
        # 最短経路候補の中で一番コストの少ないノードを選ぶ、あとその時のコストも
        min_cost = INF
        for i in range(N):
            if Color[i] != 2 and D[i] < min_cost:
                # 訪問済みでなく、最小コストを更新したら
                min_cost = D[i]
                u = i
        # uには次に確定するノードが入っている

        if min_cost == INF:
            # これ以上辺がない or すべて訪問済みなら
            break

        Color[u] = 2  # uは訪問済み これから最短経路木に突っ込む作業をする

        # 以下のforではsからの最短経路木にuを追加したときの更新、uまわりで次の最短経路になる候補の更新をしている
        # for v in range(N):
        #     if Color[v] != 2 and adj_mat[u][v] != INF:
        # 以上の二行は隣接リストを用いると以下のように書き換えられる
        for idx_adj_u, w_adj_u in adj[u]:
            if Color[idx_adj_u] != 2:
                # 訪問済みでなく、u→vへの経路が存在するならば
                if D[u] + w_adj_u < D[idx_adj_u]:  # ココがprimと異なる点である
                    # 今までのs→vへの距離の候補(D[v])よりも、
                    # 新しいuを用いた経路(D[u] + M[u][v]はs→u→vの距離を示す)のほうが小さければ更新する
                    D[idx_adj_u] = D[u] + w_adj_u  # u周りにおける最短経路の候補の更新
                    P[idx_adj_u] = u  # sからの最短経路木の更新
                    Color[idx_adj_u] = 1  # 訪問経験ありに更新(この処理に関しては意味はない)

    return D, P


D, P = dijkstra(adj, 0)

for i in range(N):
    print(i, D[i])
