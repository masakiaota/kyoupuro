# http://poj.org/problem?id=3255
# ムズすぎか？
from collections import defaultdict
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


N = 4
R = 4
graph = defaultdict(lambda: [])
graph[0] = [(1, 100)]
graph[1] = [(0, 100), (2, 250), (3, 200)]
graph[2] = [(1, 250), (3, 100)]
graph[3] = [(1, 250), (2, 100)]

# 2番目の最短距離まで持つDijkstra法
# s→tの2番目の最短距離は、s→uの1番目の最短距離と2番目の最短距離にu→tの距離を足した中に候補がある
# 各点について1,2番目の最短距離を持ってdijkstraを行えばよい

pq = PriorityQueue([])
dist = [float('inf')] * N  # 1番目の最短距離
dist2 = [float('inf')] * N  # 2番目の最短距離

dist[0] = 0
pq.push((0, 0))  # (距離, ノード)  # 1番目の交差点からスタートする
while pq:
    d, v = pq.pop()
    if dist2[v] < d:  # 扱っている距離が知りたい距離より大きいならなにもしない
        continue
    for to, cost in graph[v]:
        d_to = d + cost
        if dist[to] > d_to:  # 最短距離更新
            dist[to], d_to = d_to, dist[to]
            pq.push((dist[to], to))

        # 2番目の距離更新(1番目よりは大きくて、2番目よりは小さいものが更新対象)
        if dist2[to] > d_to and dist[to] < d_to:
            dist2[to] = d_to
            pq.push((dist2[to], to))

print(dist[N - 1])
print(dist2[N - 1])
