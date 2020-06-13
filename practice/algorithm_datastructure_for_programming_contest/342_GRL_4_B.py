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
def topological_sort(dag, indeg):
    '''
    dag ... default dictで定義された隣接リスト(DAGを想定)
    indeg ... 各ノードについての流入量のリスト(inputのときについでにやったほうが計算量若干少なく済むでしょ？本質じゃないけど)
    '''
    is_visited = [False] * len(indeg)  # 訪問間利用
    ret = []  # グラフをソートしたあとに返す答え
    perm = [-1] * len(indeg)  # できる処理を一気にしようとしたときの順番

    que = deque()  # 各要素はノード, 処理回数
    for i, deg in enumerate(indeg):
        if deg == 0:
            que.append((i, 0))
            is_visited[i] = True

    while que:
        u, cnt = que.popleft()  # uは流入量0のノード
        ret.append(u)  # なので答えに加えていく
        perm[u] = cnt
        for to in dag[u]:  # uに隣接するノードtoについて深さ優先探索
            indeg[to] -= 1  # 隣接するノードは流入量を減らす
            if indeg[to] or is_visited[to]:
                continue  # 流入待ちか訪問済みだったら飛ばす
            que.append((to, cnt + 1))  # 次の訪問候補に追加
            is_visited[to] = True

    if False in is_visited:
        print(is_visited)
        raise ValueError('inputted graph is not DAG')

    return ret, perm


ans, _ = topological_sort(adj, indeg)
print(*ans, sep='\n')
