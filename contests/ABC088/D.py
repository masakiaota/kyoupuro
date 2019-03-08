from collections import deque

H, W = list(map(int, input().split()))


masu = [input() for _ in range(H)]

# search for kuro
kuro = 0
for row in masu:
    for s in row:
        if s == '#':
            kuro += 1


# search for minimum dist by bfs
que = deque([(0, 0, 0)])
move = [(1, 0), (-1, 0), (0, 1), (0, -1)]
visited = [[False for _ in range(W)] for _ in range(H)]

# print(len(visited[0]), len(visited))

while (que):
    i, j, cost = que.popleft()
    # print(i, j)

    visited[j][i] = True
    if (j == H - 1) and (i == W - 1):
        break
    for movex, movey in move:
        tmpx, tmpy = i + movex, j + movey
        # visited[tmpy][tmpx] = True

        # if (tmpx < 0) or(tmpx >= W) or (tmpy < 0) or (tmpy >= H):
        if not(tmpx in range(W)) or not(tmpy in range(H)):
            continue

        if (masu[tmpy][tmpx] == '.') and not(visited[tmpy][tmpx]):
            que.append((i + movex, j + movey, cost + 1))
            visited[tmpy][tmpx] = True
else:
    print(-1)
    exit()
# print(sum(visited[4]))
print(H * W - kuro - cost - 1)
