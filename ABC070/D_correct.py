N = int(input())
X, Y = [], []

tree = {key+1: [] for key in range(N)}  # 木構造はノードをキーにする辞書にしておく
# 必要なのは、あるノードを指定したときに、隣り合うノードと、コスト
#[(隣のノード, コスト)]の順で格納する

for n in range(N - 1):
    a, b, c = list(map(int, input().split()))
    tree[a].append((b, c))
    tree[b].append((a, c))  # 方向に依存しない格納

# for n in tree.keys():
#     tree[n] = list(set(tree[n]))  # 重複を削除
# こいつが原因ではない

Q, K = list(map(int, input().split()))

cost = {}  # Kがkeyに行くまでにかかるコストの辞書


def dfs(v, p, d):
    """
    深さ優先探索
    再帰的に処理する
    args:
        v: 現在の頂点
        p: vの親(訪問元)
        d: 現在の距離
    """
    cost[v] = d
    for i, co in tree[v]:
        if i == p:
            continue
        dfs(i, v, d + co)


dfs(K, -1, 0)

print(cost)
ans = []
for q in range(Q):
    x, y = list(map(int, input().split()))
    ans.append(cost[x] + cost[y])

for a in ans:
    print(a)
