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


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


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
        # 歩いて
        for di, dj in ((0, 1), (1, 0), (0, -1), (-1, 0)):
            ni, nj = i + di, j + dj
            if not (0 <= ni < H and 0 <= nj < W) or D[ni][nj] != -1 or grid[ni][nj] == 1:
                continue
            D[ni][nj] = c
            heappush(que, (c, ni, nj))
        # ワープする
        for di, dj in product([-2, -1, 0, 1, 2], repeat=2):
            ni, nj = i + di, j + dj
            if not (0 <= ni < H and 0 <= nj < W) or D[ni][nj] != -1 or ni == nj == 0 or grid[ni][nj] == 1:
                continue
            nc = c + 1
            D[ni][nj] = nc
            heappush(que, (nc, ni, nj))
    return D


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b

# もし歩くだけなら簡単
# ワープをどう処理する？
# もし周囲5*5を候補にするなら24*10**6 んー間に合うか微妙


H, W = ints()
si, sj = mina(*ints())
ti, tj = mina(*ints())
S = read_map_as(H)
D = grid_dijkstra(S, si, sj)
print(D[ti][tj])
print(*D, sep='\n')

# unionfindで連結成分切り出してから、連結成分をまたぐときだけ+1するdfsかな?
