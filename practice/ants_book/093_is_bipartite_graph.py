
from collections import deque


def is_bipartite_graph(graph, N):
    '''隣接リスト形式の入力を仮定'''
    # 再帰をしたくない(pypyの再帰は遅い)のでここでは幅優先探索で書く
    color = [-1] * N  # -1は無色。0,1で色を塗り分ける
    que = deque([(0, 0)])  # (ノード、色)
    color[0] = 0
    while que:
        u, c = que.popleft()
        for nx in graph[u]:
            if color[nx] == -1:  # 無色には色を塗る
                color[nx] = 1 - c
                que.append((nx, 1 - c))
            else:  # すでに色があるものに関しては、矛盾してないこと確認
                if color[nx] == c:
                    return False
    return True


# 入力例1
graph = {
    0: [1, 2],
    1: [0, 2],
    2: [0, 1],
}

print('Yes' if is_bipartite_graph(graph, 3) else 'No')

# 入力例2
graph = {
    0: [1, 3],
    1: [0, 2],
    2: [1, 3],
    3: [0, 2],
}

print('Yes' if is_bipartite_graph(graph, 4) else 'No')
