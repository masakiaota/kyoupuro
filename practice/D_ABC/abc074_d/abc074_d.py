# https://atcoder.jp/contests/abc074/tasks/arc083_b

# 考えること
# 1, どうやったらu→vの最短経路を復元できるか？
# 2, そのような道路が存在しない場合とはどういうときか？

# 重要な考察
# 1, iを固定したとき、j=arg_j min D[i][j] はiと必ず隣接する
# 2, d(u,v)=d(u,i)+d(i,v) を満たすiは最短経路の中にある。

# minimum sppaning treeからスタートする発想は悪くなかったねぇ、発想が逆だったけど

# 解説見ました...

# はじめにAを全点間グラフに見立てて、異なる3点u,v,iの関係性について考える。
# もしA(u,v)>A(u,i)+A(i,v)となる組み合わせがあるならば、明らかに前提条件に矛盾する -1
# そうでないならば、Aの全点間グラフは都市を結ぶ道路となり得る。ここから最短経路辺の抽出を考える。
# 最短経路辺の抽出=最短でない辺を外して良い
# A(u,v)=A(u,i)+A(i,v)となる組み合わせの場合、辺(u,v)を外せる ∵経由しても最短になるから、そっちを使う。代わりに最短でない辺は外して良い
# A(u,v)<A(u,i)+A(i,v)となる組み合わせの場合、辺は外せない ∵この辺を外すと与えられたAに矛盾


import sys
read = sys.stdin.readline
ra = range
enu = enumerate


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


N = read_a_int()
A = read_matrix(N)
from itertools import combinations
edge_rm = set()
for u, v in combinations(range(N), 2):
    for i in range(N):
        if i == u or i == v:
            continue
        a = A[u][i]
        b = A[i][v]
        c = A[u][v]
        # print(a, b, c)
        if c == a + b:
            edge_rm.add((u, v) if u < v else(v, u))
        elif c > a + b:
            print(-1)
            exit()
# print(edge_rm)
ans_sum = 0
for a in A:
    for aa in a:
        ans_sum += aa
ans_sum //= 2
for u, v in edge_rm:
    ans_sum -= A[u][v]
print(ans_sum)
