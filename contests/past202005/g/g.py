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


MOD = 10**9 + 7
INF = 2**63  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 素直なbfsですかね


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
        for di, dj in ((1, 1), (0, 1), (-1, +1), (1, 0), (-1, 0), (0, -1)):
            ni, nj = i + di, j + dj
            if not (0 <= ni < H and 0 <= nj < W) or D[ni][nj] != -1:
                continue
            nc = c + grid[ni][nj]
            D[ni][nj] = nc
            heappush(que, (nc, ni, nj))
    return D


N, X, Y = read_ints()
# 無限に広がるとは言ってるものの
# 余分に402^2とれば十分

shift = 201
grid = [[1] * (2 * shift + 1) for _ in range(2 * shift + 1)]  # 余分に取ること忘れずに
for _ in range(N):
    x, y = read_ints()
    grid[x + shift][y + shift] = INF

D = grid_dijkstra(grid, shift, shift)
# print(*D, sep='\n')
print(D[X + shift][Y + shift] if D[X + shift][Y + shift] < INF else -1)
