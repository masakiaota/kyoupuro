# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


# 時間がなくてやけくそで木の直径
from collections import deque, defaultdict
Tree = defaultdict(lambda: [])

# input data
N, u, v = read_ints()
for _ in range(N - 1):
    s, t = list(map(int, input().split()))
    w = 1
    Tree[s - 1].append((t - 1, w))
    Tree[t - 1].append((s - 1, w))


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

print(max(Dists) - 1)
