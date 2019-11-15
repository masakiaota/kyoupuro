# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/all/GRL_4_B
# 仕組みは単純ゆえに、本では解説がバッサリ省略されている。操作の可視化を行うと図のようになる。
# ここでは隣接リストを用いて実装する。

from collections import deque
# 隣接リストはdefaultdictで管理すると楽です。
from collections import defaultdict
adj = defaultdict(lambda: [])

# input data
n_V, n_E = list(map(int, input().split()))
indeg = [0] * n_V  # 流入量管理
for _ in range(n_E):
    s, t = list(map(int, input().split()))
    indeg[t] += 1  # 流入量更新
    adj[s].append(t)


# Topological Sort by BFS
def topological_sort(adj, indeg):
    '''
    adj ... default dictで定義された隣接リスト
    indeg ... 各ノードについての流入量のリスト(inputのときについでにやったほうが計算量若干少なく済むでしょ？本質じゃないけど)
    '''
    is_visited = [False] * len(indeg)  # 訪問間利用
    ret = []  # グラフをソートしたあとに返す答え

    def bfs(s):  # bfsを定義する
        # 与えた始点からbfsしていく関数。
        que = deque([s])
        is_visited[s] = True
        while que:
            u = que.popleft()  # uは流入量0のノード
            ret.append(u)  # なので答えに加えていく

            for v in adj[u]:  # uに隣接するノードvについて深さ優先探索
                indeg[v] -= 1  # 隣接するノードは流入量を減らす
                if (indeg[v] == 0) and (not is_visited[v]):  # 未訪問かつ流入0のノードだったら
                    que.append(v)  # 次の訪問候補に追加
                    is_visited[v] = True

    # 初期bfsを駆動させるための初期(?)ループ
    for u in range(n_V):  # すべてのノードについて
        if (indeg[u] == 0) and (not is_visited[u]):
            # 流入0かつ未訪問
            bfs(u)

    return ret


ans = topological_sort(adj, indeg)
print(*ans, sep='\n')
