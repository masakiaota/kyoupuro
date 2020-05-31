# https://www.slideshare.net/hcpc_hokudai/advanced-dp-2016 動的計画法の問題の解説がされている 神

# これが比較的わかりやすいかも https://algo-logic.info/bit-dp/

'''
定式化(本は"集める"DPで定義してるが、わかりやすさのため"配る"DPで定式化)

ノーテーション
S ... 頂点集合
| ... 和集合演算子
dp[S][v] ... 重みの総和の最小。頂点0から頂点集合Sを経由してvに到達する。

更新則
dp[S|{v}] = min{dp[S][u]+d(u,v)} ただしv∉S

初期条件
dp[∅][0] = 0 #Vはあらゆる集合
dp[V][u] = INF #ほかはINFで初期化しておく

答え
dp[すべての要素][0] ... 0からスタートしてすべての要素を使って最後に0に戻るための最小コスト
'''

# verify https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/all/DPL_2_A
INF = 2 ** 31
from itertools import product


def solve(n, graph):
    '''nは頂点数、graphは隣接行列形式'''
    max_S = 1 << n  # n個のbitを用意するため
    dp = [[INF] * n for _ in range(max_S)]
    dp[0][0] = 0
    for S in range(max_S):
        for u, v in product(range(n), repeat=2):
            if (S >> v) & 1 or u == v:  # vが訪問済みの場合は飛ばす
                continue
            dp[S | (1 << v)][v] = min(dp[S | (1 << v)][v],
                                      dp[S][u] + graph[u][v])

            # # 別解 #集めるDPの発想
            # if u == v or (S >> v) & 1 == 0:  # Sにはvが含まれている必要がある
            #     continue
            # dp[S][v] = min(dp[S][v],
            #                dp[S - (1 << v)][u] + graph[u][v])
    print(dp[-1][0] if dp[-1][0] != INF else -1)


# # 入力例
# n = 5
# graph = [[INF, 3, INF, 4, INF],
#          [INF, INF, 5, INF, INF],
#          [4, INF, INF, 5, INF],
#          [INF, INF, INF, 0, 3],
#          [7, 6, INF, INF, INF]]
# solve(n, graph)


# verify用
n, e = map(int, input().split())
graph = [[INF] * n for _ in range(n)]
for _ in range(e):
    s, t, d = map(int, input().split())
    graph[s][t] = d
solve(n, graph)
