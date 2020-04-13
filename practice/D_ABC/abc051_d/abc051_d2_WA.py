# https://atcoder.jp/contests/abc051/tasks/abc051_d
# 考えたこと
# 1.warshal floydからの経路復元でやってみる←御解答
# s→tまでは最短経路が2つ存在する場合がある。そのときうまくいかない

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate
from copy import deepcopy


def read_ints():
    return list(map(int, read().split()))


from collections import defaultdict
INF = 10**9
N, M = read_ints()
D = [[INF] * N for _ in range(N)]

edges = defaultdict(lambda: INF)
for _ in range(M):
    a, b, c = read_ints()
    a -= 1
    b -= 1
    D[a][b] = c
    D[b][a] = c
    edges[b, a] = c
    edges[a, b] = c
for i in range(N):
    D[i][i] = 0

# warshal floyd
for k in range(N):
    for i in range(N):
        if D[i][k] >= INF:  # 定数倍高速化
            # ノード同士がつながってなければ更新しない
            continue
        for j in range(N):
            D[i][j] = min(D[i][j], D[i][k] + D[k][j])


# 経路復元


def restore_path(s: int, t: int, edges: dict, D: list):
    '''
    s→tまでの最短経路を復元する
    edgesはオリジナルの辺のリスト edges[a,b]=a,bを結ぶ辺の長さ。ないときは(INF、自分自身へもINF)
    Dは最短経路D[s][t]=s→tの最短経路
    '''
    ret = []  # 経路
    cur = s
    while cur != t:
        for i in range(N):  # Nはノード数 = len(D) or D.shape[0]
            if i != cur and edges[cur, i] + D[i][t] == D[cur][t]:
                ret.append((cur, i) if cur < i else (i, cur))  # 大小関係をはっきりさせておく
                cur = i
                break
    return ret


#
from itertools import combinations
edges_used = set()
for s, t in combinations(range(N), 2):
    edges_used |= set(restore_path(s, t, edges, D))
print(M - len(edges_used))
