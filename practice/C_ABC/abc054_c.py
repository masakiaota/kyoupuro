# https://atcoder.jp/contests/abc054/tasks/abc054_c
# なんか水パフォだけど全探索するだけ

import sys
read = sys.stdin.readline


from itertools import permutations


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
from collections import defaultdict
graph = defaultdict(lambda: [])
for m in range(M):
    a, b = read_ints()
    graph[a].append(b)
    graph[b].append(a)


def is_ok(p):  # 順序pにしたがってグラフがつながっているか
    for now, nx in zip(p, p[1:]):
        if nx not in graph[now]:
            return 0
    return 1


ans = 0
for perm in permutations(range(2, N + 1)):
    perm = [1] + list(perm)
    ans += is_ok(perm)

print(ans)
