# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/11/ALDS1_11_C
# 実家のような安心感
# 与えられるノード番号についてはすべて0based indexに治す

from collections import deque

# データの入力
N = int(input())
DG = {}
for i in range(N):
    tmp = input().split()
    if int(tmp[1]) != 0:
        DG[i] = [int(x)-1 for x in tmp[2:]]
    else:
        DG[i] = []


result = {}
is_visited = {key: False for key in range(N)}
# bfs
# 初期queue
que = deque([(0, 0)])  # 必要なものはnodeidと原点からの距離
while que:
    nodeid, cost = que.popleft()  # 先入れ先出し
    result[nodeid] = cost
    is_visited[nodeid] = True  # この文は最初の1for目しか意味ないんだけどなんかうまい方法ないのか
    for nextnode in DG[nodeid]:
        if is_visited[nextnode]:
            pass
        else:  # 未訪問のものについては探索候補に入れる
            que.append((nextnode, cost + 1))
            is_visited[nextnode] = True  # 探索候補に入れた瞬間に訪問管理しないと重複を生じる場合がある

for i in range(N):
    if is_visited[i]:
        print(i + 1, result[i])
    else:
        print(i+1, -1)
