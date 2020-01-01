# グラフ理論で橋や関節点などが出てきたとき用
# ノーテーションは螺旋本P349に準じる

# 自己ループと二重辺を含まないN頂点M辺の無向連結グラフGがdictの形式で与えられているとする
# 以下例
N, M = list(map(int, input().split()))
G = defaultdict(list)
for _ in range(M):
    a, b = list(map(int, input().split()))
    G[a].append(b)
    G[b].append(a)

import sys
sys.setrecursionlimit(2**16 - 1)
from collections import defaultdict
INF = 2**31

# prenumとlowestの更新
prenum = defaultdict(lambda: None)
parent = defaultdict(lambda: None)
lowest = defaultdict(lambda: None)
timer = 0  # dfsに入った順番を記録(prenum用)
is_visited = defaultdict(lambda: False)  # 木を作るために循環してはいけない
root = 0


def dfs(cur, prev):  # 次に訪問するノードが以前のノードprevではないことを識別する必要があるので引数がこうなっている
    # 関数に入るときにprenumとparentを確定してく
    global timer
    prenum[cur] = lowest[cur] = timer
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


dfs(root, None)  # rootが問題ないか再度確認を


# 橋の数を探す例
# 螺旋本の用語を使えば、prenum[p]<lowest[u]となるedge(p,u)が橋
ans = 0
for u, p in parent.items():
    if p is None:
        continue
    if prenum[p] < lowest[u]:
        ans += 1
print(ans)


# 関節点を探す例 #バグがあるかも
# 間接点の重複を許したくないので集合型で管理
ret = set()
n_p_is_root = 0  # 螺旋本でいうところのnp。rootから2つ以上の子が存在するならば、rootは間接点
# 0を根にしているのでその親はNoneであり、これを考慮する必要はない
for u, p in parent.items():
    if p is None:
        # rootの親はないので
        continue
    elif p == root:
        # uはあるノード。pはその親.
        n_p_is_root += 1
    if prenum[p] <= lowest[u]:
        ret.add(p)

if n_p_is_root == 1:
    # rootが間接点でないならroot(0)を消す
    ret.remove(root)

print(*ret)
