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

# s→k→tまでの最短経路はどうやってみ見つける?
# 3点s,k,tについてmin(D[u,s]+D[u,k]+D[u,t])がs→k→tとなるときの最小距離


H, W = read_ints()
A = read_matrix(H)


def grid_dijkstra(grid, si: int, sj: int):
    '''grid上のdijkstra法。gridはそこに入るときにかかるコスト
    si,sj は開始の座標。'''
    from heapq import heappop, heappush
    H = len(grid)
    W = len(grid[0])
    D = [[-1] * W for _ in [0] * H]  # -1がINFを意味する
    que = [(0, si, sj)]
    D[si][sj] = 0
    while que:
        c, i, j = heappop(que)
        for di, dj in ((0, 1), (1, 0), (0, -1), (-1, 0)):
            ni, nj = i + di, j + dj
            if not (0 <= ni < H and 0 <= nj < W) or D[ni][nj] != -1:
                continue
            nc = c + grid[ni][nj]
            D[ni][nj] = nc
            heappush(que, (nc, ni, nj))
    return D


min_costs1 = grid_dijkstra(A, H - 1, 0)
min_costs2 = grid_dijkstra(A, 0, W - 1)
min_costs3 = grid_dijkstra(A, H - 1, W - 1)
ans = INF
for i, j in product(range(H), range(W)):
    tmp = min_costs1[i][j] + min_costs2[i][j] + min_costs3[i][j] - 2 * A[i][j]
    ans = min(ans, tmp)


print(ans)
