# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/1/DPL_1_A
# めちゃくちゃナップサックっぽい。ってかナップサックじゃん
# 本ではdpテーブルに当たるTを1次元配列にすることによってメモリを削減しているが、ここではしない。

# dpテーブルは初期化と、更新が必要でそれらを意識しながら実装する。
# C ... (m,) i番目のコインの額面
# dp ... (m, n+1) i番目まで(それを含む)のコインをつかってj円払うときのコインの最小枚数

INF = 1e5

# load data
n, m = list(map(int, input().split()))
C = list(map(int, input().split()))

# dpテーブルの作成 minを取りながら更新するときは初期値にINFをぶちこんでおくとよい
dp = [[INF] * (n + 1) for _ in range(m)]  # (m,n+1)のリストの作成

# dpテーブルの初期化
# dp[:][0] は常に0 ∵ 0円はコインを一つも使わなければ良い
# dp[0][:] は即座に決定する。 ∵ dp[0][j]%C[0] == 0ならばdp[0][j]//C[0]の枚数で j 円を達成可能だから
for i in range(m):
    dp[i][0] = 0
for j in range(1 + n):
    dp[0][j] = j  # いまC[0]は必ず1と問題文にあるので、これを用いることにする。

# dpテーブルの更新
# 更新則は T[i][j]=min(T[i-1][j], T[i][j-C[i]]+1)
# 1項目 i-1までのコインでj円が作れるとき、i番目のコインを使わなくてもj円を作る枚数は変わらない。
# 2項目 i番目のコインを使ってちょうどj円を達成するには、j-C[i]円のときに達成可能なコインの枚数に1を足せば良い
# 最小のコインの枚数を知りたいのだから上記の小さい方を採用すれば良い
from itertools import product
# Z字にdpを更新していく.初期値のある列と行は省略
for i, j in product(range(1, m), range(1, n + 1)):
    # 配列外アクセスを防ぐためにminを取る操作を二回に分けている
    dp[i][j] = min(dp[i][j], dp[i - 1][j])
    if 0 <= j - C[i]:
        dp[i][j] = min(dp[i][j], dp[i][j - C[i]] + 1)

print(dp[i][j])
