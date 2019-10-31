# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/11/ALDS1_11_D

from collections import defaultdict
import sys
sys.setrecursionlimit(1 << 25)  # 再起上限引き上げ

G = defaultdict(lambda: [])

# load data
N, M = list(map(int, input().split()))
color = {key: None for key in range(N)}  # 色というかグループ #intで表す #Noneは未訪問を示す
# graphをdefault dictで保持することにする。
for _ in range(M):
    s, t = list(map(int, input().split()))
    G[s].append(t)
    G[t].append(s)

# 離接リストをもとに、たどり着ける友達関係をグループ化する #(union findが使える気もするけどここではDFSで領域を探索していく)


def dfs(node: int, co: int):
    # 終了条件 #すでに訪問済みなら訪問しない
    if color[node] is not None:
        return
    # 処理
    color[node] = co
    # 探索
    for n in G[node]:
        dfs(n, co)
    return


# すべてのnodeを開始点としてdfsで領域わけしてみる
c = 1
for node in G.keys():
    dfs(node, c)  # 訪問済みだったら色を塗り替えないというないようはdfs関数内の処理で吸収できる
    c += 1


# 問題に回答していく
Q = int(input())
for _ in range(Q):
    s, t = list(map(int, input().split()))
    if ((color[s] is not None) or (color[t] is not None)) and (color[s] == color[t]):
        # s,tも友達がいて、かつ同じグループだったら
        print('yes')
    else:
        print('no')
