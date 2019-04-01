# 王道の幅優先探索。
# 下記のリンクの問題を例にコピペで済むようにここに記す。
# https://abc007.contest.atcoder.jp/tasks/abc007_3


from collections import deque


def readln():
    return list(map(int, input().split()))


H, W = readln()
sy, sx = readln()
gy, gx = readln()
(sy, sx, gy, gx) = (x-1 for x in (sy, sx, gy, gx))
C = [input() for _ in range(H)]

# 探索の準備
mvx = (1, 0, -1, 0)
mvy = (0, 1, 0, -1)
visited = [[False] * W for _ in range(H)]
# queueにスタックさせる配列の内容は考えておこう
# 今回ならば、(探索するx,y,今までの移動距離)を入れておけばよいだろう。
que = deque([(sy, sx, 0)])
visited[sy][sx] = True  # queに入ったところは訪れるのが確定している。
# 幅優先探索
while que:
    y, x, cost = que.popleft()

    if y == gy and x == gx:
        ans = cost
        break

    for dy, dx in zip(mvy, mvx):
        y_new, x_new = y + dy, x + dx
        # 探索に含めない条件
        if not (-1 < y_new < H) or not (-1 < x_new < W):
            continue
        if visited[y_new][x_new]:
            continue
        if C[y_new][x_new] == '#':
            continue
        # 探索に追加してあげる
        que.append((y_new, x_new, cost + 1))
        visited[y_new][x_new] = True

print(ans)
