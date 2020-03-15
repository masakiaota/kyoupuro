# https://atcoder.jp/contests/abc021/tasks/abc021_c
# 面白い

# ダイクストラして自分の家から各点への最短経路を求めておく
# D[a,b]が初期の距離。幅優先探索で距離-1を満たすノードを取得して通りの数にかけていく。

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

from scipy.sparse import csr_matrix, lil_matrix
# 変換する場合はcsrを。あとから数字を入れる場合はlilを使うと効率的
# ダイクストラ法 (正の単一始点最短経路) (全点間最短経路)
from scipy.sparse.csgraph import dijkstra
MOD = 10**9 + 7
N = read_a_int()
a, b = read_ints()
a -= 1
b -= 1
M = read_a_int()
adj_mat = lil_matrix((N, N))
G = [list() for _ in range(N)]  # 隣接リスト形式でも持っておく
for _ in range(M):
    x, y = read_ints()
    x -= 1
    y -= 1
    adj_mat[x, y] = 1
    adj_mat[y, x] = 1
    G[y].append(x)
    G[x].append(y)

D = dijkstra(adj_mat, directed=False, indices=b)

# 幅優先探索 しながら答えを加算する
d = D[a]
que = deque([(a, d)])
ans = 1
is_visited = [False] * N  # 忘れがち
is_visited[a] = True

while que:
    now, d = que.popleft()
    # 終了条件 d==0
    if d == 0:
        break
    cnt = 0
    for nx in G[now]:
        if D[nx] != d - 1:
            continue
        cnt += 1  # カウントには含めるけど探索はしないものもある
        if is_visited[nx]:
            continue
        is_visited[nx] = True
        que.append((nx, d - 1))
    ans *= cnt
    if ans >= MOD:
        ans %= MOD

print(ans)
