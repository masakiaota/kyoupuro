# 重要な考察 : 現在位置から1マス上が黒になる場合、現在位置を必ず踏む必要がある
# 詳しい話は蟻本で

# 主に必要なデータと関数
# 1. あるマスについて踏んだかどうかを示す配列
# 2. 踏んだかどうかの情報からそのマスが黒か判断する関数
# 3. 1(0)行目の踏み方を全探索する関数
# 4. 牛の踏み方が決まったときに何回踏んだかを返す関数

# 入力

M = 4
N = 4
tile = [[1, 0, 0, 1],
        [0, 1, 1, 0],
        [0, 1, 1, 0],
        [1, 0, 0, 1]]


from itertools import product
from copy import deepcopy


opt = None  # 最適な盤面の保存
min_flip = 2**31  # 最小のひっくり返す回数


def get_color(x, y, flip):  # 踏んだかどうかの情報からそのマスが黒か判断する関数
    c = tile[x][y]
    # 周りの踏んだ状況を取得
    for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
        nx, ny = x + dx, y + dy
        if 0 <= nx < M and 0 <= ny < N:
            c ^= flip[nx][ny]
    return c


def generate_flip_0row():  # 辞書順で0行目の踏み方をbit全探索するやつ
    for ret in product(range(2), repeat=M):
        yield list(ret)


def simulate(flip):  # 最適な踏み方をシミュレートする
    # flipはすでに0行目が埋まっている前提
    for i, j in product(range(1, M), range(N)):
        if get_color(i - 1, j, flip):  # もし上のタイルが黒ならこのタイルは踏まないと上のタイルを白にできない
            flip[i][j] = 1
    # 有効な踏み方か？つまりM-1行目がすべて白になっているかチェック
    for j in range(N):
        if get_color(M - 1, j, flip):
            return -1  # もし黒があれば強制的に終了

    return sum([sum(x) for x in flip])  # flipした回数


# 実装する
for zeroth_row in generate_flip_0row():
    flip = [[0] * N for _ in range(M)]  # 踏んだマス
    flip[0] = zeroth_row
    tmp = simulate(flip)
    if tmp != -1 and tmp < min_flip:
        opt = deepcopy(flip)
        min_flip = tmp

if opt == None:
    print('IMPOSSIBLE')
else:
    print(*opt, sep='\n')
