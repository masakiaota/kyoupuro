# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/all/GRL_5_A
# シンプルなアルゴリズム
# 厳密な証明は知らないけど、直感的に良さそう

from collections import defaultdict
Tree = defaultdict(lambda: [])

# input data
N = int(input())
for _ in range(N):
    s, t, w = list(map(int, input().split()))
    Tree[s].append((t, w))
    Tree[t].append((s, w))

# dfsで実装したい(疲れたのでまた今度。)
