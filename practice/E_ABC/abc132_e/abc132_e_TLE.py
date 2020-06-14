# https://atcoder.jp/contests/abc132/tasks/abc132_e
# ぱっと思いつくのはダイクストラで3回で到達できる点を探索していくとかかなぁ

# 最悪ループ回数((13*10**5))程度だと思うんだけどTLEする

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret


    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため
from heapq import heapify, heappop, heappush, heappushpop


def bfs(graph, s, N):
    '''
    graph...隣接リスト形式 リスト内要素はノード
    s...始点ノード
    N...頂点数

    return
    ----------
    D ... 各点までの最短距離
    '''
    # pq = PriorityQueue([])
    cnt = 0
    pq = deque([])
    D = [2**31] * N
    D[s] = 0
    is_visited = [False] * N
    is_visited[s] = True
    pq.append((D[s], s))  # (最短距離, 次のノード)
    while pq:
        d, v = pq.popleft()
        # 探索するのはgraph[v]じゃなくてケンケンパしてたどり着くことのできるノード
        for to in next_node(v, graph):
            cnt += 1
            assert cnt < 10**7  # 10**7じゃなくてもTLEになっている...
            if is_visited[to]:
                continue
            cost = 1
            D[to] = d + cost
            is_visited[to] = True
            pq.append((D[to], to))
    return D


def next_node(v, graph):
    # 距離3のノードの集合を集めてくる
    ret = set()
    q = deque([(0, v)])
    while q:
        d, u = q.popleft()
        if d == 3:
            ret.add(u)
            continue
        for to in graph[u]:
            q.append((d + 1, to))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N, M = read_ints()
graph = defaultdict(lambda: [])
for _ in range(M):
    u, v = mina(*read_ints())
    graph[u].append(v)
S, T = mina(*read_ints())

D = bfs(graph, S, N)
print(D[T] if D[T] != INF else -1)
