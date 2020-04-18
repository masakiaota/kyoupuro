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

H, W = read_ints()
A = read_matrix(H)


mv = [(1, 0), (-1, 0), (0, 1), (0, -1), ]


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


min_costs = bfs(A, H - 1, 0)
ans1 = min_costs[H - 1][W - 1]

# re constact load
# 再帰的に考えると、i,jにおいてmin_cost[i,j]-A[i,j]となるmin_costを周囲から探して行けばok


# import numpy as np
# print(np.array(min_costs))
# print()
# print(np.array(A))


is_visited = [[False] * W for _ in range(H)]

candi_t = []


def dfs(i, j):
    # print(i, j)
    is_visited[i][j] = True
    for di, dj in mv:
        ni, nj = i + di, j + dj
        if not (0 <= ni < H and 0 <= nj < W) or is_visited[ni][nj]:
            continue
        if min_costs[ni][nj] == min_costs[i][j] - A[i][j]:
            dfs(ni, nj)
    A[i][j] = 0
    candi_t.append((i, j))  # ここまでたどり着ければ残りはコスト0で移動できる


dfs(H - 1, W - 1)  # reconstract

min_costs2 = bfs(A, 0, W - 1)
ans2 = min_costs2[H - 1][W - 1]

# for i, j in candi_t:
#     print(i, j, min_costs2[i][j])
#     ans2 = min(ans2, min_costs2[i][j])

# print()
# print(np.array(A))
# print()
# print(np.array(min_costs2))

print(ans1 + ans2)
# print(ans1, ans2)

# 実装後ちゃってきた


# 戦略1
# s→k の最小コストを保持、その経路(全部)を0にする。そのt→kにおいてその経路に一個でも差し掛かればコスト無しで移動できるA

# s→tまでの最短経路を考えたときに、それらの経路をメモしておく(すすべての候補)の
# kから候補の地点に行く最短経路の中で一番最短を足すのがマスの整備に必要な最短コスト

