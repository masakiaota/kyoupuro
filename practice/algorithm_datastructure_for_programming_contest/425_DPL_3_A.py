# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/all/DPL_3_A
# P426の解説がわかりやすい。なるほどなぁという感じ。

from itertools import product, chain

# load data
H, W = list(map(int, input().split()))
C = []
for i in range(H):
    C.append(list(map(int, input().split())))

# dp (H,W) ... dp[i][j]においてはC[i][j]から左上に向かってできる最大の正方形の辺の長さを記録していく

# dpテーブルの作成
dp = [[-1] * W for _ in range(H)]  # (H,W)

# dpテーブルの初期化
# 今回は0行目と0列目が即座にわかる
for j in range(W):
    if C[0][j] == 1:
        dp[0][j] = 0
    else:
        dp[0][j] = 1

for i in range(H):
    if C[i][0] == 1:
        dp[i][0] = 0
    else:
        dp[i][0] = 1


# dpテーブルの更新
for i, j in product(range(1, H), range(1, W)):
    if C[i][j] == 1:
        dp[i][j] = 0
    else:
        dp[i][j] = min(dp[i - 1][j - 1], dp[i - 1][j], dp[i][j - 1]) + 1

side_length = max(chain.from_iterable(dp))  # flatten
print(side_length**2)
