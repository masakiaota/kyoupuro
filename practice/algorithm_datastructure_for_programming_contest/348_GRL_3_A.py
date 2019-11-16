# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/all/GRL_3_A
# P350に図による説明はあるのだが端折られているところもあり、少し苦しんだ。
# 関節点を求めるには主に2パートに分かれる
# 1. dfsによる各ノードのprenum(深さ優先探索でたどる順番), parent(深さ優先探索でたどる木構造), lowest(こいつはわかりにくいので求め方を図解した)
# 2. 以上で求めた情報に基づいて、そのノードが関節点か決定する。

# 1について`dfs()`で求めて
# 2について`art_points()`で求める
INF = 10**6 + 1
import sys
sys.setrecursionlimit(2**16 - 1)
from collections import defaultdict
G = defaultdict(lambda: [])
# input data
n_V, n_E = list(map(int, input().split()))

for _ in range(n_E):
    s, t = list(map(int, input().split()))
    G[s].append(t)
    G[t].append(s)


def get_art_point(G, n_V):
    # graph Gがdefaultdictで定義された隣接リストであること前提とする。
    # n_Vは頂点数
    prenum = [None] * n_V
    parent = [None] * n_V
    lowest = [INF] * n_V
    timer = 0  # dfsに入った順番を記録(prenum用)
    is_visited = [False] * n_V  # 木を作るために循環してはいけない
    root = 0

    # 1のフェーズ、prenumとparent,lowestについて確定していく。
    def dfs(cur, prev):  # 次に訪問するノードが以前のノードprevではないことを識別する必要があるので引数がこうなっている
        # 関数に入るときにprenumとparentを確定してく
        nonlocal timer  # nonlocalは指定した変数が見つかるまでスコープを広げて探す
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

    # 2のフェーズ。次にprenumとlowestに注目して間接点を決定してく
    def art_points(root):
        # 間接点の重複を許したくないので集合型で管理
        ret = set()
        n_p_is_root = root  # 螺旋本でいうところのnp。rootから2つ以上の子が存在するならば、rootは間接点
        # 0を根にしているのでその親はNoneであり、これを考慮する必要はない
        for u, p in enumerate(parent):
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
            ret.remove(0)
        return ret

    # ここから具体的な処理
    dfs(root, None)
    return art_points(root)


ans = list(get_art_point(G, n_V))
ans.sort()
if ans:
    print(*ans, sep='\n')
else:
    pass
