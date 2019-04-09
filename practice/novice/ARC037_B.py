import sys
read = sys.stdin.readline
sys.setrecursionlimit(1 << 25)


def readln():
    return list(map(int, read().split()))


N, M = readln()
visited = [False] * N  # 訪問の管理用


# デフォルト値を持つ辞書
from collections import defaultdict
edge = defaultdict(lambda: set())  # キーで指定したノードが隣接するノードを記録しておく。
for _ in range(M):
    u, v = readln()
    edge[v-1].add(u-1)
    edge[u-1].add(v-1)


def dfs(node, prev):
    '''
    現在のノードと先ほどのノードを受け取る

    閉路があった場合もその時点で0を返し続ける
    探索しきる事ができたら(木だったら)1を返す。
    '''
    if visited[node]:
        # 訪問したことがあったらアウト!
        # return0とする
        return 0

    # print(node, prev)
    visited[node] = True
    # dfs!
    for n in edge[node]:
        if n == prev:
            continue  # 前のnodeは探索する辺から抜いておく
        if dfs(n, node) == 0:
            return 0
    return 1  # 結合成分がないかすべて探索が終わったら1を返す。


ans = 0
for i in range(N):
    if not visited[i]:
        ans += dfs(i, -1)

print(ans)
