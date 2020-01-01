# https://atcoder.jp/contests/abc075/tasks/abc075_c
# グラフの橋という概念を扱う問題 #関節点と似た概念

# 螺旋本の用語を使えば、prenum[p]<lowest[u]となるedge(p,u)が橋

import sys
sys.setrecursionlimit(2**16 - 1)
from collections import defaultdict

INF = 10**9
N, M = list(map(int, input().split()))
G = defaultdict(list)
for _ in range(M):
    a, b = list(map(int, input().split()))
    G[a].append(b)
    G[b].append(a)

# graph Gがdefaultdictで定義された隣接リストであること前提とする。
prenum = defaultdict(lambda: None)
parent = defaultdict(lambda: None)
lowest = defaultdict(lambda: None)
timer = 0  # dfsに入った順番を記録(prenum用)
is_visited = defaultdict(lambda: False)  # 木を作るために循環してはいけない
root = 0

# 1のフェーズ、prenumとparent,lowestについて確定していく。


def dfs(cur, prev):  # 次に訪問するノードが以前のノードprevではないことを識別する必要があるので引数がこうなっている
    # 関数に入るときにprenumとparentを確定してく
    global timer
    prenum[cur] = lowest[cur] = timer  # 教科書の説明とは違ってprenumは0スタートだが本質的な違いはない
    timer += 1
    parent[cur] = prev
    is_visited[cur] = True

    # 次のノードに訪問していく
    for nx in G[cur]:  # nxは次に探索すべきノード
        if (not is_visited[nx]):
            dfs(nx, cur)
            lowest[cur] = min(lowest[cur], lowest[nx]
                              )  # 子のlowestの最小値が取得できる
        elif nx != prev:  # 探索済みにつながる木以外の経路 いわゆるback-edge
            # back-edge先のノードの方とどっちが小さいか比較
            lowest[cur] = min(lowest[cur], prenum[nx])


dfs(1, None)

# 2のフェーズ bredgeを見つけていく
# 螺旋本の用語を使えば、prenum[p]<lowest[u]となるedge(p,u)が橋
ans = 0
for u, p in parent.items():
    if p is None:
        continue
    if prenum[p] < lowest[u]:
        ans += 1
print(ans)
