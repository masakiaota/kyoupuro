# https://atcoder.jp/contests/joi2011yo/tasks/joi2011yo_e

# 問題文を読み間違えて無限に時間を溶かしたのはもったいない！よく問題を読もうね！

from collections import deque
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read() for _ in range(H)]


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


H, W, N = read_ints()
MAP = read_map(H)

# S(0)と数字の座標を格納しておくリスト
n_yx = [0] * (N + 1)
for i, row in enumerate(MAP):
    for j, m in enumerate(row):
        if m == 'S':
            n_yx[0] = (i, j)
            continue

        for k in range(1, N + 1):
            if m == str(k):
                n_yx[k] = (i, j)
                break
        # 探索の準備
mvx = (1, 0, -1, 0)
mvy = (0, 1, 0, -1)


def bfs(s, g, n_s):
    '''
    sの座標を受け取ってgの座標にたどり着く最短距離を返す
    ただしn_sに従って、それより大きい数字のマスは移動することができない。
    '''
    sy, sx = s
    gy, gx = g

    visited = [[False] * W for _ in range(H)]
    # queueにスタックさせる配列の内容は考えておこう
    # 今回ならば、(探索するx,y,今までの移動距離)を入れておけばよいだろう。
    que = deque([(sy, sx, 0)])
    visited[sy][sx] = True  # queに入ったところは訪れるのが確定している。
    # 幅優先探索
    while que:
        y, x, cost = que.popleft()

        if y == gy and x == gx:
            return cost

        for dy, dx in zip(mvy, mvx):
            y_new, x_new = y + dy, x + dx
            # 探索に含めない条件
            if not (-1 < y_new < H) or not (-1 < x_new < W):
                continue
            if visited[y_new][x_new]:
                continue

            if MAP[y_new][x_new] == 'X':
                continue

            # 探索に追加してあげる
            que.append((y_new, x_new, cost + 1))
            visited[y_new][x_new] = True

    print('something wrong')


ans = 0
for i in range(N):
    # print(i)
    ans += bfs(n_yx[i], n_yx[i+1], i)
    # print(ans)
print(ans)

'''
.X...X.S.X
6..5X..X1X
...XXXX..X
X..9X...X.
8.X2X..X3X
...XX.X4..
XX....7X..
X..X..XX..
X...X.XX..
..X.......
'''
