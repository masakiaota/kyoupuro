# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/10/ALDS1_10_B
# 普通に難しい
# アイデアは写真の解説を参照

# データの入力
N = int(input())

for i in range(N):
    if i == 0:
        P = list(map(int, input().split()))
        continue
    P.append(int(input().split()[1]))

# dpテーブルの用意 minを取るときはinf埋めしておくと処理が楽になるしバグが見つかりやすい
INF = 2**32-1
dp = [[INF]*N for _ in range(N)]

# dpテーブルの初期化
# 対角成分0
for ij in range(N):
    dp[ij][ij] = 0

# dpの更新 どう更新しているのか、なんでこう更新するのかは写真を参照
for l in range(1, N):  # lは対角成分からいくつ離れているか (なん個行列をかけるかという解釈も可)
    for i in range(N):
        j = i+l
        if j >= N:
            continue
        for k in range(i, j):  # 最小値探索
            dp[i][j] = min(dp[i][k] + dp[k+1][j] + P[i]
                           * P[j+1]*P[k+1], dp[i][j])
# from pprint import pprint
# pprint(dp)
print(dp[0][-1])
