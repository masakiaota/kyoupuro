import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


def read_map_as(H, replace={'#': 1, '.': 0}, pad=None):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    if pad is None:
        ret = []
        for _ in range(H):
            ret.append([replace[s] for s in read()[:-1]])
            # 内包表記はpypyでは若干遅いことに注意
            # #numpy使うだろうからこれを残しておくけど
    else:  # paddingする
        ret = [[pad] * (W + 2)]  # Wはどっかで定義しておくことに注意
        for _ in range(H):
            ret.append([pad] + [replace[s] for s in read()[:-1]] + [pad])
        ret.append([pad] * (W + 2))

    return ret


def grid_dijkstra(grid, si: int, sj: int):  # TLEでした
    '''grid上のdijkstra法。gridはそこに入るときにかかるコスト
    si,sj は開始の座標。'''
    from heapq import heappop, heappush
    H = len(grid)
    W = len(grid[0])
    D = [[-1] * W for _ in [0] * H]  # -1がINFを意味する
    que = [(0, si, sj)]
    while que:
        c, i, j = heappop(que)
        if D[i][j] != -1:
            continue
        D[i][j] = c
        for di, dj in product([-2, -1, 0, 1, 2], repeat=2):
            ni, nj = i + di, j + dj
            if not (0 <= ni < H and 0 <= nj < W) or D[ni][nj] != -1 or ni == nj == 0 or grid[ni][nj] == 1:
                continue
            if (di == 0 and abs(dj) == 1) or (dj == 0 and abs(di) == 1):  # 歩いて行ける
                nc = c
            else:  # ワープして行ける
                nc = c + 1
            heappush(que, (nc, ni, nj))
    return D


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from itertools import product, permutations, combinations
from collections import deque

# もし歩くだけなら簡単
# ワープをどう処理する？
# もし周囲5*5を候補にするなら24*10**6 んー間に合うか微妙


H, W = ints()
si, sj = mina(*ints())
ti, tj = mina(*ints())

S = read_map_as(H)
# 01bfsを実装
D = [[-1] * W for _ in [0] * H]  # -1がINFを意味する
que = deque([(0, si, sj)])
while que:
    c, i, j = que.popleft()
    if D[i][j] != -1:
        continue
    D[i][j] = c
    for di, dj in product([-2, -1, 0, 1, 2], repeat=2):
        ni, nj = i + di, j + dj
        if not (0 <= ni < H and 0 <= nj < W) or D[ni][nj] != -1 or ni == nj == 0 or S[ni][nj] == 1:
            continue
        if (di == 0 and abs(dj) == 1) or (dj == 0 and abs(di) == 1):  # 歩いて行ける
            que.appendleft((c, ni, nj))  # コストが低いので先に出したい
        else:  # ワープして行ける
            que.append((c + 1, ni, nj))  # コストが高いのであとに出したい

print(D[ti][tj])
# print(*D, sep='\n')
