# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
"""
グラフの貼り付け用部品。BFS、Dijkstra、UnionFind はそれぞれ独立している。
使う部品だけを main.py にコピーする。AHC073の問題仕様は未記入。
"""


# BFS: graph[v] は v から進む頂点のリスト。未到達距離は -1。
# 例: graph = [[1, 2], [2], []]; dist = bfs(graph, 0)
def bfs(graph, start):
    from collections import deque

    dist = [-1] * len(graph)
    dist[start] = 0
    queue = deque([start])
    while queue:
        v = queue.popleft()
        next_dist = dist[v] + 1
        for to in graph[v]:
            if dist[to] == -1:
                dist[to] = next_dist
                queue.append(to)
    return dist


# Dijkstra: graph[v] は (to, cost) のリスト。cost は0以上。未到達は inf。
# 例: graph = [[(1, 4), (2, 1)], [(2, 2)], []]; dist = dijkstra(graph, 0)
def dijkstra(graph, start):
    import heapq

    dist = [float("inf")] * len(graph)
    dist[start] = 0
    heap = [(0, start)]
    while heap:
        current_dist, v = heapq.heappop(heap)
        if current_dist != dist[v]:
            continue
        for to, cost in graph[v]:
            new_dist = current_dist + cost
            if new_dist < dist[to]:
                dist[to] = new_dist
                heapq.heappush(heap, (new_dist, to))
    return dist


# UnionFind: uf.union(a, b), uf.same(a, b), uf.size(a) を使う。
# 例: uf = UnionFind(n); uf.union(0, 1); connected = uf.same(0, 1)
class UnionFind:
    __slots__ = ("parent",)

    def __init__(self, n):
        self.parent = [-1] * n

    def find(self, x):
        root = x
        while self.parent[root] >= 0:
            root = self.parent[root]
        while x != root:
            parent = self.parent[x]
            self.parent[x] = root
            x = parent
        return root

    def union(self, a, b):
        a = self.find(a)
        b = self.find(b)
        if a == b:
            return False
        if self.parent[a] > self.parent[b]:
            a, b = b, a
        self.parent[a] += self.parent[b]
        self.parent[b] = a
        return True

    def same(self, a, b):
        return self.find(a) == self.find(b)

    def size(self, x):
        return -self.parent[self.find(x)]
