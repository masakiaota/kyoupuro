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

N, X, Y = read_ints()
X -= 1
Y -= 1

# 一箇所だけ短絡している
# もし短絡していなかったら簡単
# O(NN)でも間に合いそう

# 愚直にダイクストラしてもええか？
from scipy.sparse import csr_matrix  # 自分で配列を作ってからcsrに入れよう(lilに打ち込んでいくのは非常に遅い)

# ダイクストラ法 (正の単一始点最短経路) (全点間最短経路)
from scipy.sparse.csgraph import dijkstra
graph = [[0] * N for _ in range(N)]
for i in range(N - 1):
    graph[i][i + 1] = 1
    graph[i + 1][i] = 1
graph[X][Y] = 1
graph[Y][X] = 1
adj_mat = csr_matrix(graph)
D = dijkstra(adj_mat, directed=False)
cnt = Counter(D.reshape(-1))
for i in range(1, N):
    print(int(cnt[i]) // 2)


# D[i,j]でi→jに向かう最短経路
# indicesで始点を指定している場合は、D[j]でok
