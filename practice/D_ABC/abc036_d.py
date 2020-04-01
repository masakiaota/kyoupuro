# https://atcoder.jp/contests/abc036/tasks/abc036_d
# 難しかった！けど解けた！
# 木の葉の方からDPする(構造上dfsのほうがいいかな)
# f(u)を、u以下の部分木におけるuが白だった場合の条件を満たす通りの総数
# g(u)を、u以下の部分木におけるuが黒だった場合の条件を満たす通りの総数 と定義する
# すると木構造上でuの子Cに対して以下の漸化式がなりたつ。
# f(u)= \prod_{c \in C}(f(c)+g(c)) ∵uが白なら、今まで条件を満たすすべてのパターンはuでも条件を満たす(足し算)。また子が複数あっても部分木同士では独立なので組み合わせは掛け算
# g(u)= \prod_{c \in C}(f(c)) ∵uが黒ならば、子が黒の場合条件を満たさなくなってしまう(g(c)は数え上げない)
# よって再帰関数などでこの漸化式を解けば良い。

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline

MOD = 10**9 + 7
# default import
from collections import defaultdict


def read_ints():
    return list(map(int, read().split()))


N = int(input())
Tree = defaultdict(lambda: [])
for _ in range(N - 1):
    a, b = read_ints()
    Tree[a].append(b)
    Tree[b].append(a)


def dfs(u, p):
    '''
    uとその親pを受け取って
    f(u),g(u)を返す
    '''
    fu = gu = 1
    adj = Tree[u]
    # 終了条件
    if len(adj) == 1 and adj[0] == p:
        return 1, 1
    # 更新
    for c in adj:
        if c == p:  # 親は探索しない
            continue
        fc, gc = dfs(c, u)
        fu *= fc + gc
        gu *= fc
    fu %= MOD
    gu %= MOD
    return fu, gu


f, g = dfs(1, 0)
# print(f, g)
print((f + g) % MOD)
