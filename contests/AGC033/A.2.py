# 入力が10**5とかになったときに100ms程度早い
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


def read_map(H, W):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    tmp = ['#'*(W+2)]
    tmp = tmp + ['#' + read()[:-1] + '#' for _ in range(H)]
    tmp = tmp + ['#' * (W + 2)]
    return tmp


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


H, W = read_ints()
A = read_map(H, W)
visited = [[True]*(W+2)]
visited.extend([[True] + [False] * W + [True] for _ in range(H)])
visited.append([True]*(W+2))  # 無駄な探索を省くため
from itertools import product
from collections import deque
# print(visited)
# 開始地点
start = []
for i, a in enumerate(A):
    if i == 0 or i == H + 1:
        continue
    for j, aa in enumerate(a):
        if j == 0 or j == W + 1:
            continue
        if aa == '#':
            start.append((i, j))

# 探索の準備
mvx = (1, 0, -1, 0)
mvy = (0, 1, 0, -1)


# distlist = []

# for si, sj in start:
n_white = (W)*(H)
# visited = [[False] * (W+2) for _ in range(H+2)]
que = deque([(si, sj, 0) for si, sj in start])
for si, sj in start:
    n_white -= 1
    visited[si][sj] = True

# 幅優先探索
while que:
    y, x, cost = que.popleft()

    if n_white <= 0:
        # ans = cost
        break
    for dy, dx in zip(mvy, mvx):
        y_new, x_new = y + dy, x + dx
        # 探索に含めない条件
        if not (-1 < y_new < H+2) or not (-1 < x_new < W+2):
            continue
        if visited[y_new][x_new]:
            continue

        # 探索に追加してあげる
        que.append((y_new, x_new, cost + 1))
        visited[y_new][x_new] = True
        n_white -= 1
        if n_white == 0:
            ans = cost + 1
            break

    # A[si][sj] = '#'
print(ans+1)
