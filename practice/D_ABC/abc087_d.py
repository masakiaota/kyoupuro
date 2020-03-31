# https://atcoder.jp/contests/abc087/tasks/arc090_b
# ノードL→RへのコストはDであるとすれば、有向グラフと見なせる
# 任意のノードからbfsをして座標をメモしていけば良い。
# 探索済みのノードに到達したときに座標があっているかチェック、あっていればそれ以上探索する必要はない。

# 連結していないグラフがあるかもしれない。(だけど連結していないグラフはお互いに独立)
# 0以上10**9以下っていう制約なのでそこでWAを出されるかと思ったがそんなことなかった


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


from collections import defaultdict, deque

N, M = read_ints()
graph = defaultdict(lambda: [])  # 有向グラフ、隣接リスト形式
for _ in range(M):  # データ読み込み
    l, r, d = read_ints()
    l -= 1
    r -= 1
    graph[l].append((r, d))
    graph[r].append((l, -d))

# bfs
X = [None] * N  # visitedも兼ねる


def bfs(u):  # スタートするノードを入れる
    que = deque([(u, 0)])  # (今のノード,今の座標)
    X[u] = 0
    while que:
        u, x = que.popleft()
        # print(u, x)
        for nu, add in graph[u]:
            nx = x + add
            if X[nu] == nx:
                continue
            # 次の探索
            if X[nu] is not None and X[nu] != nx:
                print('No')
                exit()
            X[nu] = nx
            que.append((nu, nx))


for u in range(N):
    if X[u] is None:
        bfs(u)
print('Yes')
