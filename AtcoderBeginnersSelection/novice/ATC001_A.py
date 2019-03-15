from itertools import product
import sys
sys.setrecursionlimit(1 << 25)


def readln():
    return list(map(int, input().split()))


H, W = readln()
C = [input() for _ in range(H)]

# スタート位置の設定
for i, j in product(range(H), range(W)):
    if C[i][j] == 's':
        start = (i, j)

# 上下左右の探索用
mv = [
    (1, 0),
    (0, -1),
    (-1, 0),
    (0, 1)
]

# 訪問したかの管理
is_visited = [[False for _ in range(W)] for _ in range(H)]


# 再帰関数によるdfsの実装
# 到達できたらTrueを返すようにする
# 行けないマスに関してはreturn Falseですぐさま終了
# goalの場合はTrueを返し、Trueを保持したまま再帰が帰ってくるようにする。
def dfs(i, j):
    # スコア等を引数に入れていくと便利なときもある
    if not (0 <= i < H) or not (0 <= j < W):
        # 迷路の外
        return False
    if is_visited[i][j] or C[i][j] == '#':
        # 訪問済み もしくは 壁
        return False

    is_visited[i][j] = True
    if C[i][j] == 'g':
        return True
    # 4方向の再起探索
    for di, dj in mv:
        flg = dfs(i + di, j + dj)
        if flg:
            # trueが帰ってきたら再起のために返す
            return True
    # Trueを返さなかった場合はgにたどり着かなかったということなので、falseを返す
    return False


if dfs(*start):
    print('Yes')
else:
    print('No')
