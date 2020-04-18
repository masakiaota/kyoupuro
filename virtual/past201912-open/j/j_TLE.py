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


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    # →1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**63  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# s→tまでの最短経路はどうやってみ見つける?
# grid bfs で 各点への最短距離
# スタートから各店への最短距離たどってすでにならした地面はコスト0に置換
# 再びgrid bfsで各店への最短距離を出す

# TLEになるんやがw

H, W = read_ints()
A = read_matrix(H)


mv = {(0, 1), (1, 0), (0, -1), (-1, 0)}


def bfs(A, si, sj):
    min_costs = [[INF] * W for _ in range(H)]  # is_visited代わりでもある
    que = deque([(si, sj, 0)])  # (スタートの座標i,j,cost)
    min_costs[si][sj] = 0
    while que:
        i, j, c = que.popleft()  # cにはi,jまでの最小コスト

        for di, dj in mv:
            ni, nj = i + di, j + dj
            if not (0 <= ni < H and 0 <= nj < W):
                continue
            nc = c + A[ni][nj]
            if min_costs[ni][nj] <= nc:  # >は通す
                continue
            min_costs[ni][nj] = nc
            que.append((ni, nj, nc))
    return min_costs


min_costs1 = bfs(A, H - 1, 0)
min_costs2 = bfs(A, 0, W - 1)
min_costs3 = bfs(A, H - 1, W - 1)
ans = INF
for i, j in product(range(H), range(W)):
    tmp = min_costs1[i][j] + min_costs2[i][j] + min_costs3[i][j] - 2 * A[i][j]
    ans = min(ans, tmp)


print(ans)
