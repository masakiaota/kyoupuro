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


def get_sortest_path(P, t):
    '''P...各要素に親番号を記録した最短経路木
    t...目的ノード'''
    # !未verify
    path = []
    while t is not None:
        path.append(t)
        t = P[t]
    return path  # 逆順
