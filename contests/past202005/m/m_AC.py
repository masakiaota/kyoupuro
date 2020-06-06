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


def bfs(graph, s, N):
    '''
    graph...隣接リスト形式 リスト内要素は(ノード, エッジ長)
    s...始点ノード
    N...頂点数

    return
    ----------
    D ... 各点までの最短距離
    '''
    pq = deque()
    D = [INF] * N
    D[s] = 0
    pq.append((D[s], s))  # (最短距離, 次のノード)
    while pq:
        _, v = pq.popleft()
        for to, cost in graph[v]:
            if D[to] != INF:  # v周りにおける最短経路の候補の更新
                continue
            D[to] = D[v] + cost
            pq.append((D[to], to))
    return D


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

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
for ss in starts[:-1]:
    D = bfs(graph, ss, N)
    Ds.append(D)

graph = [[0] * (K + 1) for _ in range(K + 1)]
# print(Ds)
for ia, a in enu(starts):
    for ib, b in enu(starts):
        if ia == ib:
            continue
        if ia == K:
            graph[ia][ib] = Ds[ib][a]
        else:
            graph[ia][ib] = Ds[ia][b]

# print(graph)

# こっから巡回セールスマン問題
n = K + 1
max_S = 1 << n  # n個のbitを用意するため
dp = [[INF] * n for _ in range(max_S)]
dp[0][0] = 0
for S in range(max_S):
    for u, v in product(range(n), repeat=2):
        if (S >> v) & 1 or u == v:  # vが訪問済みの場合は飛ばす
            continue
        dp[S | (1 << v)][v] = min(dp[S | (1 << v)][v],
                                  dp[S][u] + graph[u][v])

    # # 別解 #集めるDPの発想
    # if u == v or (S >> v) & 1 == 0:  # Sにはvが含まれている必要がある
    #     continue
    # dp[S][v] = min(dp[S][v],
    #                dp[S - (1 << v)][u] + graph[u][v])

print(min(dp[(max_S - 1) - 1]))
# print(*dp, sep='\n')
