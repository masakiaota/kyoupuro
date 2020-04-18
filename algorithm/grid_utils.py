# グリッド系の問題用の関数
import sys
sys.setrecursionlimit(1 << 25)
ra = range
enu = enumerate

read = sys.stdin.readline


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as(H, replace={'#': 1, '.': 0}):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    ret = []
    for _ in range(H):
        ret.append([replace[s] for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


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
