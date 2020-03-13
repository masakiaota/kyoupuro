# https://atcoder.jp/contests/abc073/tasks/abc073_d

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))



# default import
from itertools import product, permutations, combinations

from scipy.sparse import csr_matrix
from scipy.sparse.csgraph import dijkstra

# ダイクストラか？
# 全点間の最小距離を取得
# rについてpermutationして。その通りに訪れたときの距離をシミュレーション
# 最小値を選ぶ

N, M, r = read_ints()
R = [x - 1 for x in read_ints()]
adj_mat = [[0] * N for _ in range(N)]
for _ in range(M):
    a, b, c = read_ints()
    a -= 1
    b -= 1
    adj_mat[a][b] = c
    adj_mat[b][a] = c

D = dijkstra(csr_matrix(adj_mat, dtype='int'), directed=False)

# 全探索パート
ans = 2 ** 31


def get_kyori(p):
    ret = 0
    for ps, pt in zip(p[:-1], p[1:]):
        ret += D[ps, pt]
    return ret


for p in permutations(R):
    ans = min(ans, get_kyori(p))
print(int(ans))
