# https://atcoder.jp/contests/abc054/tasks/abc054_c
# なんか水パフォだけどdfs書くだけじゃない？じゃないわむずいわ
# 全探索か？

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
from collections import defaultdict
graph = defaultdict(lambda: [])
for m in range(M):
    a, b = read_ints()
    graph[a].append(b)
    graph[b].append(a)

# ans = 0


def dfs(now, pre):  # 今いるノードnowから来たノードpreを省いて次の探索を行う
    # 終了条件
    if len(graph[now]) == 1:
        return 1

    ret = 0
