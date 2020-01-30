# https://atcoder.jp/contests/abc138/tasks/abc138_d
# 下に数を最初に合計してから深さ優先探索なり、幅優先探索なりで下にカウンターを伝播させていけば良い。
# 累積和の木構造バージョン
# 注意としては、ノードa<bで与えられるときでも頂点aがbの親とは限らない。


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


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


# 再帰の上限を緩和する(引数は適当)
import sys
sys.setrecursionlimit(1 << 25)
from collections import defaultdict

N, Q = read_ints()
child = defaultdict(lambda: [])  # ノード0は存在しない
score = [0] * (N + 1)
for _ in range(N - 1):
    a, b = read_ints()
    child[a].append(b)
    child[b].append(a)
for _ in range(Q):
    p, x = read_ints()
    score[p] += x

# 木を走査しながらscoreを合計していく
ans = [0] * (N + 1)


def dfs(p, u, s):  # ノードuの親pにおけるスコアs
    s += score[u]
    ans[u] = s
    for c in child[u]:
        if p == c:
            continue
        dfs(u, c, s)


dfs(-1, 1, 0)
print(*ans[1:])
