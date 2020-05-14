import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret


    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため
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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc087/tasks/arc090_b
# x0を0に固定したときに各点への最短距離が求められて、それが矛盾しなければ良さそう
# ここがまず違う。一番端がまずわからないし複数ある可能性もあるやで

N, M = read_ints()
graph = defaultdict(lambda: [])
LRD = []
deg_in = [0] * N
for _ in range(M):
    l, r, d = read_ints()
    l -= 1
    r -= 1
    LRD.append((l, r, d))
    deg_in[r] += 1
    graph[l].append((r, d))
    graph[r].append((l, d))

for start, d_i in enu(deg_in):
    if d_i != 0:
        continue
    D, _ = dijkstra(graph, start, N)


D, _ = dijkstra(graph, 0, N)
start = D.index(max(D))
D, _ = dijkstra(graph, start, N)  # 一番左をスタートにしないと行けない #一番遠いノードかな
# すべての入力と矛盾しないか確認
# print(start, D)
for l, r, d in LRD:
    if abs(D[r] - D[l]) != d:
        print('No')
        exit()
print('Yes')
