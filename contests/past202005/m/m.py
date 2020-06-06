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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 最小シュタイナー木？
# 個々の街からの最短距離はわかる
N, M = read_ints()
graph = defaultdict(lambda: [])
for _ in ra(M):
    u, v = read_ints()
    u -= 1
    v -= 1
    graph[u].append((v, 1))
    graph[v].append((u, 1))

s = read_a_int() - 1
K = read_a_int()
T = read_ints()
starts = [s] + [t - 1 for t in T]
Ds = []
for ss in starts:
    D, P = dijkstra(graph, ss, N)
    Ds.append(D)


# 16!の全探索ができないのが問題
# greedyでやってみるしか無いよね
is_visited = [False] * len(starts)
is_visited[0] = True
now = 0  # スタートから
ans = 0
for _ in range(len(starts) - 1):
    to_det = -1
    d = INF
    for to in range(len(starts)):  # 目的地
        if is_visited[to]:
            continue
        if Ds[now][starts[to]] <= d:
            d = Ds[now][starts[to]]
            to_det = to
    # print(d)
    is_visited[to_det] = True
    now = to_det
    ans += d
print(ans)
# 半分ぐらいWAか...
#
