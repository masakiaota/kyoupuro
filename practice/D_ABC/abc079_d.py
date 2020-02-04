# https://atcoder.jp/contests/abc079/tasks/abc079_d

# 各数字を1に書き換えるときの最小のコストを保持しておけば良い。
# どうやって最小のコストを保持する？→ダイクストラ的なdpを実装すれば良い
from scipy.sparse.csgraph import dijkstra
from scipy.sparse import csr_matrix, lil_matrix
import numpy as np
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


H, W = read_ints()
C = csr_matrix(read_matrix(10))
A = read_matrix(H)

# ダイクストラDFでi→1の各点への最短経路を計算する。
D = dijkstra(C, directed=True)

ans = 0
for a in A:
    for aa in a:
        if abs(aa) == 1:
            continue
        ans += D[aa][1]

print(int(ans))
