# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/all/GRL_5_A
# 全点間の距離を求めているとO(n**2)でパソコンが爆発する。
# P354はO(n)のシンプルなアルゴリズム。
# 厳密な証明は知らないけど、直感的に良さそう

from collections import deque, defaultdict
Tree = defaultdict(lambda: [])

# input data
N = int(input())
for _ in range(N - 1):
    s, t, w = list(map(int, input().split()))
    Tree[s].append((t, w))
    Tree[t].append((s, w))


def bfs(Tree, n_V, s):
    '''
    指定した点sからの単一始点で各点までの距離を返す。
    n_Vは頂点の数
    Treeはdefault dictで、valueには(隣接ノード,そこまでのコスト)がリスト形式で格納されているとする。
    '''
    INF = 10**9
    Dists = [INF] * n_V  # 距離の初期化 #こいつを更新して返すことにする
    is_visited = [False] * n_V
    is_visited[s] = True
    que = deque([(s, 0)])  # (ノード番号,そこまでたどり着くためのコスト)
    while que:
        cur, cost = que.popleft()
        Dists[cur] = cost
        for nx_node, nx_cost in Tree[cur]:
            if is_visited[nx_node]:
                continue
            que.append((nx_node, nx_cost + cost))
            is_visited[nx_node] = True

    return Dists


# 任意の点から一番遠い点を求めて、そこからさらに一番遠い点がほしい
Dists = bfs(Tree, N, 0)
next_node = Dists.index(max(Dists))  # numpyでいうargmaxしてる
Dists = bfs(Tree, N, next_node)

print(max(Dists))
