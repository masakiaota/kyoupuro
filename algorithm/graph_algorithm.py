####################ダイクストラ####################
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


def dijkstra(graph, s, N):
    '''
    graph...隣接リスト形式 リスト内要素は(ノード, エッジ長)
    s...始点ノード
    N...頂点数

    return
    ----------
    D ... 各点までの最短距離
    P ... 最短経路木における親
    '''
    pq = PriorityQueue([])
    P = [None] * N
    D = [float('inf')] * N
    D[s] = 0
    pq.push((D[s], s))  # (最短距離, 次のノード)
    while pq:
        d, v = pq.pop()
        if D[v] < d:  # この辺を扱っても最短距離にならない
            continue  # is_visitedなくてもこれを使うことで最小のものを再び探索するのを防げる
        for to, cost in graph[v]:
            if D[to] > D[v] + cost:  # v周りにおける最短経路の候補の更新
                D[to] = D[v] + cost
                pq.push((D[to], to))
                P[to] = v
    return D, P


####################クラスカル####################
class UnionFind:
    def __init__(self, N):
        self.N = N  # ノード数
        # 親ノードをしめす。負は自身が親ということ。
        self.parent = [-1] * N  # idxが各ノードに対応。

    def root(self, A):
        # print(A)
        # ノード番号を受け取って一番上の親ノードの番号を帰す
        if (self.parent[A] < 0):
            return A
        self.parent[A] = self.root(self.parent[A])  # 経由したノードすべての親を上書き
        return self.parent[A]

    def size(self, A):
        # ノード番号を受け取って、そのノードが含まれている集合のサイズを返す。
        return -self.parent[self.root(A)]

    def unite(self, A, B):
        # ノード番号を2つ受け取って、そのノード同士をつなげる処理を行う。
        # 引数のノードを直接つなぐ代わりに、親同士を連結する処理にする。
        A = self.root(A)
        B = self.root(B)

        # すでにくっついている場合
        if (A == B):
            return False

        # 大きい方に小さい方をくっつけたほうが処理が軽いので大小比較
        if (self.size(A) < self.size(B)):
            A, B = B, A

        # くっつける
        self.parent[A] += self.parent[B]  # sizeの更新
        self.parent[B] = A  # self.rootが呼び出されればBにくっついてるノードもすべて親がAだと上書きされる

        return True

    def is_in_same(self, A, B):
        return self.root(A) == self.root(B)


def kruskal(N, edges):
    '''Nは頂点数,edgesは(長さ,s,t)を要素に持つリスト
    最小全域森のtotal_cost、最小全域森を構成する辺の集合used_edgesを返す'''
    uf = UnionFind(N)
    total_cost = 0
    used_edges = []
    edges = sorted(edges)
    for c, s, t in edges:
        if uf.is_in_same(s, t):
            continue
        uf.unite(s, t)
        total_cost += c
        used_edges.append((c, s, t))
    return total_cost, used_edges

####################ベルマンフォード Bellman Ford####################


def bellman_ford(edges, s, N):
    '''
    edges ... (cost,from,to)を各要素に持つリスト
    s...始点ノード
    N...頂点数

    return
    ----------
    D ... 各点までの最短距離
    P ... 最短経路木における親
    '''
    P = [None] * N
    inf = float('inf')
    D = [inf] * N
    D[s] = 0
    for n in range(N):  # N-1回で十分だけど、N回目にもアップデートがあったらnegative loopを検出できる
        update = False  # 早期終了用
        for c, ot, to in edges:
            if D[ot] != inf and D[to] > D[ot] + c:
                update = True
                D[to] = D[ot] + c
                P[to] = ot
        if not update:
            break  # 早期終了
        if n == len(edges) - 1:
            raise ValueError('NegativeCycleError')
    return D, P

####################utils####################


def graph_to_edges(graph):
    '''(cosr,from,to)を要素に持つリストに変換'''
    # edgeへの変換
    edges = []
    for ot, v in graph.items():
        for to, c in v:
            edges.append((c, ot, to))
    return edges


def get_sortest_path(P, t):
    # 最短経路木経路復元
    '''P...各要素に親番号を記録した最短経路木
    t...目的ノード'''
    # !未verify
    path = []
    while t is not None:
        path.append(t)
        t = P[t]
    return path  # 逆順


def dist_on_tree(tree, s, N):
    # 木上の最短距離(地味に書くことが多い)
    '''tree...(to,cost)の隣接リスト形式
    s...始点ノード
    N...ノード数'''
    from collections import deque
    D = [-1] * N
    D[s] = 0
    que = deque([(s, 0)])  # 現在のノード、そこまでの距離
    while que:
        now, dist = que.popleft()
        for to, cost in tree[now]:
            if D[to] != -1:
                continue
            D[to] = dist + cost
            que.append((to, dist + cost))
    return D
