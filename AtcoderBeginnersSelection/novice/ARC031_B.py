# すべてのマスを探索することになってもたかだか10*10回のループで終わる。
# どのマスを埋めるべきかすべて探索(xのマスを一つずつ埋めてみる)
# 一つ埋めたときに、埋めた島がつながっているかを判別するのにここでは練習のため深さ優先探索を用いる
# ちなみに埋めたときに埋めた島はすべてのマスとつながっているはず
from itertools import product
import sys
sys.setrecursionlimit(1 << 25)


MAP = [list(input()) for _ in range(10)]

mv = [
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 0),
]


def dfs(i, j):
    # 探索したマスはvisitedにする
    # 探索不可能マスのときはFalseを返す
    # 最後は1を返す

    if not (-1 < i < 10) or not (-1 < j < 10):
        return False
    if visited[i][j]:
        return False
    if MAP[i][j] == 'x':
        return False

    visited[i][j] = True
    for di, dj in mv:
        dfs(i + di, j + dj)

    return 1


for i, j in product(range(10), range(10)):
    if MAP[i][j] == 'x':
        MAP[i][j] = 'o'
        visited = [[False for _ in range(10)] for _ in range(10)]

        n_riku = 0  # 陸の数
        for ii, jj in product(range(10), range(10)):
            if visited[ii][jj] == False and MAP[i][j] == 'o':
                n_riku += dfs(ii, jj)

        if n_riku == 1:
            # print(i, j)
            print("YES")
            exit()
        MAP[i][j] = 'x'  # もとに戻すことを忘れて闇にハマった

print('NO')
