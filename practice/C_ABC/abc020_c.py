# https://atcoder.jp/contests/abc020/tasks/abc020_c
# 素直にbfsと二分探索？ マスによってコストが違くて、最小コストを各マスにメモしていくDPもどきのBFSってなにげに初めてだな
# 譜面をグラフ構造にしてからダイクストラを適応してもできそうだけど。

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


# default import
from collections import defaultdict, Counter, deque
from itertools import product, permutations, combinations

H, W, T = read_ints()
S = read_map(H)

for i, j in product(range(H), range(W)):
    if S[i][j] == 'S':
        s = (i, j)
    if S[i][j] == 'G':
        g = (i, j)

INF = 10**11 + 10**10


def is_ok(x):
    # T秒以内にごーるできるかどうか
    mv = [(1, 0), (0, 1), (-1, 0), (0, -1)]
    dp = [[INF] * W for _ in range(H)]
    dp[s[0]][s[1]] = 0
    que = deque([(s[0], s[1], 0)])
    while que:
        i, j, t = que.popleft()
        for di, dj in mv:
            ni, nj = i + di, j + dj
            if not 0 <= ni < H or not 0 <= nj < W:
                continue
            nt = t + (x if S[ni][nj] == '#' else 1)
            if dp[ni][nj] <= nt:  # 探索する意味なし
                continue
            dp[ni][nj] = nt
            que.append((ni, nj, nt))

    return dp[g[0]][g[1]] <= T


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(10**9 + 1, 1))
