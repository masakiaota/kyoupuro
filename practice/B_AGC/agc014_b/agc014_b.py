# https://atcoder.jp/contests/agc014/tasks/agc014_b
# 条件を満たすものが存在するかの判別問題

# クエリをグラフにして考えてみる
# a-bを結ぶすべての経路は木上のa-b間の経路を必ず通る
# つまりa-b間の経路の数が偶数である必要がある。すべてのノードに対してエッジが偶数個つながっているかを確認すればよい

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()

n_edges = [0] * N
for _ in ra(M):
    a, b = read_ints()
    n_edges[a - 1] += 1
    n_edges[b - 1] += 1

for n in n_edges:
    if n & 1:
        print('NO')
        exit()
print('YES')
