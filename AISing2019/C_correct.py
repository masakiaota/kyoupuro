from collections import deque

H, W = list(map(int, input().split()))
S = [input() for _ in range(H)]

move = [(-1, 0), (1, 0), (0, -1), (0, 1)]
visited = [[False for _ in range(W)] for _ in range(H)]


def bfs(i, j):
    # print('探索from', i, j)
    # i,jで指定されたマスが属するregionに含まれる黒白の個数をかける
    if visited[i][j]:
        return 0
    else:
        cnt1, cnt2 = 0, 0
        que = deque([(i, j)])
        while que:
            # print(que)
            i, j = que.popleft()
            # print('now', i, j)
            if S[i][j] == '#':
                cnt1 += 1
            else:
                cnt2 += 1
            visited[i][j] = True
            # print(visited)
            # print(cnt1, cnt2)

            # 周りを探索
            for (dh, dw) in move:
                if ((-1 < (i + dh) < H) and (-1 < (j + dw) < W)):
                    # print(i + dh, j + dw)
                    if not visited[i + dh][j + dw]:
                        if S[i][j] != S[i + dh][j + dw]:
                            visited[i + dh][j + dw] = True
                            que.append((i + dh, j + dw))

        return cnt1 * cnt2


ans = 0
for i in range(H):
    for j in range(W):
        ans += bfs(i, j)
        # print('in for loop', i, j, ans)


print(ans)
