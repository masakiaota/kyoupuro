# https://atcoder.jp/contests/abc026/tasks/abc026_c
# 木構造のdfsで解けそう

from collections import defaultdict
from functools import lru_cache
import sys
sys.setrecursionlimit(2**20)

N = int(input())
B = defaultdict(lambda: [])
for i in range(2, N + 1):
    p = int(input())
    B[p].append(i)


@lru_cache(maxsize=2**20)
def dfs(u):  # 現在のノードが必要
    if len(B[u]) == 0:
        return 1
    mi = 10 ** 9
    ma = -10
    for nx in B[u]:
        mi = min(mi, dfs(nx))
        ma = max(ma, dfs(nx))
    return mi + ma + 1


print(dfs(1))
