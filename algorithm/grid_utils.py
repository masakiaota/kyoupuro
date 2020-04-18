# グリッド系の問題用の関数
import sys
sys.setrecursionlimit(1 << 25)
ra = range
enu = enumerate


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
