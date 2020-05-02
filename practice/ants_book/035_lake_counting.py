# http://poj.org/problem?id=2386
# union find つかったらめっちゃ簡単に解けそうな気がするやつ
#
# 練習のためdfsで
import sys
sys.setrecursionlimit(1 << 25)
from itertools import product


N = 10
M = 12
MAP = '''W........WW.
.WWW.....WWW
....WW...WW.
.........WW.
.........W..
..W......W..
.W.W.....WW.
W.W.W.....W.
.W.W......W.
..W.......W.'''


def map_as(m, replace={'W': 1, '.': 0}):
     # 入力をいい感じに0,1の二重リストにしてくれる関数
    m = m.split()
    ret = []
    for line in m:
        ret.append([replace[s] for s in line])
    return ret


MAP = map_as(MAP)


def dfs(i, j):  # 周囲を探索しながら.に置き換える,何も返さない
    # 終了条件はなくても勝手に止まる
    for di, dj in product([-1, 0, 1], repeat=2):
        ni, nj = i + di, j + dj
        if not (0 <= ni < N and 0 <= nj < M):
            continue
        if MAP[ni][nj] == 1:
            MAP[ni][nj] = 0
            dfs(ni, nj)


ans = 0
for i, j in product(range(N), range(M)):
    if MAP[i][j] == 0:
        continue
    else:
        ans += 1
        dfs(i, j)

print(ans)
