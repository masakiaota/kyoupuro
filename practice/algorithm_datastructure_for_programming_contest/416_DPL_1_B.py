# https://onlinejudge.u-aizu.ac.jp/courses/library/7/DPL/1/DPL_1_B
# 王道のナップサック問題
# 解説は螺旋本で十分わかりやすい
# また、けんちょんさんの記事もわかりやすい
# https://qiita.com/drken/items/a5e6fe22863b7992efdb#%E5%95%8F%E9%A1%8C-2%E3%83%8A%E3%83%83%E3%83%97%E3%82%B5%E3%83%83%E3%82%AF%E5%95%8F%E9%A1%8C

# 実装の都合上本とは少しだけ変えて実装する
# items (N,) ,i番目の品物の価値と重さが記録されているlist item[i]
# dp (N+1,W+1), [0,i)までのitemを考慮して大きさwのナップサックに入れる場合の価値の合計の最大値をdp[i][w]とする二次元list

# load data
N, W = list(map(int, input().split()))
items = []
for _ in range(N):
    items.append(tuple(map(int, input().split())))


# dpテーブルの作成
# 今回は最大化が目的なのですべてのマスを-1で埋めることにする
dp = [[-1] * (W + 1) for _ in range(N + 1)]

# dpテーブルの初期化
# dp[:][0]が0 ∵大きさ0のナップサックには何も入れられない
# dp[0][:]が0 ∵なにもitemを考慮しないときはナップサックに何も入らない。
for i in range(N + 1):
    dp[i][0] = 0
for w in range(W + 1):
    dp[0][w] = 0

# dpテーブルの更新
# i番目のitemが入らない場合→dp[i+1][w]=dp[i][w] ∵iを考慮しないときと価値は変わらない
# i番目のitemが入る場合→dp[i+1][w]=max(dp[i][w], dp[i][w-item[i]の重さ]+item[i]の価値)
# ∵iを入れる前の最善はdp[i][w-item[i]の重さ]にあり、それにitem[i]の価値を足すことで最善になる可能性がある。
# ただしちゃんと、iを入れないときのほうが良いかもしれないことに注意しなければいけない。
from itertools import product
for i, w in product(range(N), range(W + 1)):
    dp[i + 1][w] = dp[i][w]
    value, weight = items[i]
    if w - weight >= 0:
        dp[i + 1][w] = max(dp[i][w], dp[i][w - weight] + value)

print(dp[-1][-1])
