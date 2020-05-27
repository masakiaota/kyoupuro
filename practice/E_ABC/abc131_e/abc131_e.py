# https://atcoder.jp/contests/abc131/tasks/abc131_e

# 最短距離が2の頂点対がちょうどK個ある
# 最短距離が2の頂点対とは→うまく定式化できないな... ちょうど一つのノードを挟んで存在する

# 基本戦略、非連結からつなげてく。全結合から切っていく。一番kが多い状態から減らす
# 一番kが多い状態→ノード0を中心として、周りに1,2,3..を連結させたスターグラフ
# まわりのノードを連結していくたびにkが一つずつ少なくなるのでK=kになるまでそれを繰り返せば良さそう

from itertools import combinations
N, K = map(int, input().split())
k_max = ((N - 1) * (N - 2)) // 2
if k_max < K:
    print(-1)
    exit()

k = k_max
ans = []
for i in range(1, N):  # ここは0based
    ans.append((1, i + 1))  # ここは1basedに直す
# スターグラフ(k_maxの状態完成)
# 一つずつ引いていく
for u, v in combinations(range(1, N), r=2):
    if k == K:
        break
    k -= 1
    ans.append((u + 1, v + 1))

print(len(ans))
for u, v in ans:
    print(u, v)
