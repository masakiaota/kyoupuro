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


N = read_a_int()

from collections import defaultdict
node = defaultdict(lambda: set())  # キーで指定したノードが隣接するノードを記録しておく。
for _ in range(N-1):
    u, v, w = read_ints()
    if w % 2:
        # wじゃ奇数だったらTrue
        w = True
    else:
        w = False
    node[v-1].add((u-1, w))
    node[u-1].add((v-1, w))


# 0番目から探索していく
import numpy as np
ans = np.full(N, -1, dtype='int16')
# -1は未探索、1は黒、0は白、奇数の重みに当たるたびに反転
visited = np.full(N, False, dtype=bool)
from collections import deque
# 幅優先探索で白黒を記録していく
que = deque([(0, 0)])  # (nodeidx, 現在の色)
ans[0] = 0
visited[0] = True
while que:
    n, c = que.popleft()
    ans[n] = c

    for nex in node[n]:
        nex_node, nex_w = nex
        if visited[nex_node]:
            continue
        # 重みが奇数なら色を反転して探索に追加
        if nex_w:  # bug in this section looooool
            if c == 0:
                c = 1
            else:
                c = 0

        que.append((nex_node, c))
        visited[nex_node] = True

print(*ans, sep='\n')
