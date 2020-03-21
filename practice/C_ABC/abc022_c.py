# https://atcoder.jp/contests/abc022/tasks/abc022_c
# 1と同じノードにもう一つノードを用意してダイクストラで良くない？←ちがうな
# 一回ダイクストラしてから、1につながっているノードに対する最短経路と経路復元でごちゃごちゃやるとかは


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


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


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 1に隣接しているノードをstartとしたときのダイクストラをやる
# そのノード以外から1につながるノードでコストの最小を求める

from scipy.sparse import csr_matrix, lil_matrix
from scipy.sparse.csgraph import dijkstra, floyd_warshall, NegativeCycleError
N, M = read_ints()
graph = [[0] * N for _ in range(N)]
nebors = []
for _ in range(M):
    u, v, l = read_ints()
    u -= 1
    v -= 1
    if u == 0:
        nebors.append((v, l))
    else:
        graph[u][v] = l
        graph[v][u] = l
inf = 10**9
graph = csr_matrix(graph)
ans = inf
D = dijkstra(graph, directed=False)
for s, t in combinations(nebors, 2):
    ss, sl = s
    tt, tl = t
    ans = min(ans, sl + tl + D[ss, tt] if D[ss, tt] != 0 else inf)
print(int(ans) if ans != inf else -1)
