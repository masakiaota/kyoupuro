# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/11/ALDS1_11_B
# 本の解説通りかは知らない
# 本に書いてあったスタックを用いた深さ優先探索とは違うかも
# nodeが番号じゃなくてもいいようにdictにしちゃったけど実装が煩雑になってしまったかも

# データの読み込み
N = int(input())
DG = {}
node_ls = []
is_visited = {}
# 有向グラフをディクトで管理することにする
for _ in range(N):
    tmp = input().split()
    node_id = tmp[0]
    node_ls.append(node_id)
    is_visited[node_id] = False
    adj_num = int(tmp[1])
    if adj_num != 0:
        DG[node_id] = tmp[2:]
    else:
        DG[node_id] = []

d = {}  # 最初に訪問したときの発見時刻を記録する
f = {}  # 隣接リストを調べ終えた完了時刻を記録する
t = 1


def dfs(node):  # 必要なものは何？ 現在のノード
    # 終了条件
    global t
    if is_visited[node]:
        return  # 何もせずに上の関数に抜ける

    is_visited[node] = True
    # 発見時刻の書き込み
    d[node] = t
    t += 1  # 時刻を一つ進める
    # 再帰探索
    for no in DG[node]:
        dfs(no)
    f[node] = t
    t += 1  # 関数を終了するときにも+1する


for node in node_ls:  # 孤立しているノードもあるためこうして対処
    dfs(node)

for node in node_ls:
    print(node, d[node], f[node])
