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
    return [read()[:-1] for _ in range(H)]


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


# このグラフ構造の中で一番遠い連結成分では？
H, W = read_ints()
S = read_map(H)

# おとなしく深さ優先探索

mvx = (1, 0, -1, 0)
mvy = (0, 1, 0, -1)

from itertools import product, combinations
from collections import deque


def ret_shortest(si, sj):
    visited = [[False] * W for _ in range(H)]
    que = deque([(si, sj, 0)])
    visited[si][sj] = True

    ans = 0
    while que:
        i, j, cost = que.popleft()
        ans = max(ans, cost)
        for di, dj in zip(mvy, mvx):
            ni, nj = i + di, j + dj
            if not (-1 < ni < H) or not (-1 < nj < W):
                continue
            if S[ni][nj] == '.' and not visited[ni][nj]:
                que.append((ni, nj, cost + 1))
                visited[ni][nj] = True

    return ans


ans = 0

ans = 0
for i, j in product(range(H), range(W)):
    if S[i][j] == '.':
        ans = max(ret_shortest(i, j), ans)
print(ans)
