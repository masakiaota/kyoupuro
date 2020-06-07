# 解説→https://www.slideshare.net/hcpc_hokudai/advanced-dp-2016 18ページから

'''
蟻本と違って
dp[i][j][S]...パターンの総数。(i,j)マスまで埋めたときに、境界(埋めたマスの1つ下)がSになる場合の。

更新則
(i,j)に埋めることができないとき(例えば黒マスになってるとか、すでに埋まってるとか)
dp[i][j+1][S & ~(1<<j)] = dp[i][j][S] ∵i,jに置かない→次の境界のjbit目は必ず空白(and処理で必ず空白にする)
# S & ~(1<<j)はbin(((1<<10) - 1) & ~(1<<5))を実行してみれば正しく動作していることが確認できる。

(i,j)に縦置きを埋めるとき:
    改行が必要ないとき:
        dp[i][j+1][S|(1<<j)] += dp[i][j][S] ∵jbitは必ず埋まる
    改行が必要(つまりj==W-1のとき):
        dp[i+1][0][S|(1<<j)] += dp[i][j][S] ∵jbitは必ず埋まる


(i,j)に横置きを埋めるとき
    横が空いているとき(つまり(S>>j+1)&1==0のとき かつ j<W-1)
        dp[i][j+1][S|(1<<(j+1))] +=dp[i][j][S] ∵j+1 bitは必ず埋まる
    横が空いていないとき
        なにもしない(挿入できないので)

'''

# 入力
n = 3  # 行数
m = 3  # 列数

color = [[0, 0, 0],
         [0, 1, 0],
         [0, 0, 0]]
dp = [[[0] * (1 << m) for _ in range(m)] for _ in range(n + 1)]
dp[0][0][0] = 1  # 0,0まで埋まっているときS==0の状態のみ存在する

for i in range(n):
    for j in range(m):
        for S in range(0, 1 << m):
            if color[i][j] or (S >> j) & 1:  # おけないとき
                if j < m - 1:
                    dp[i][j + 1][S & ~(1 << j)] += dp[i][j][S]
                else:
                    dp[i + 1][0][S & ~(1 << j)] += dp[i][j][S]
                continue  # おけないので終了

            # 縦におくとき
            if j == m - 1:  # 改行
                dp[i + 1][0][S | (1 << j)] += dp[i][j][S]
            else:
                dp[i][j + 1][S | (1 << j)] += dp[i][j][S]

            # 横に置くとき
            if (S >> (j + 1)) & 1 == 0 and j < m - 1:
                dp[i][j + 1][S | (1 << (j + 1))] += dp[i][j][S]

# print(*dp, sep='\n')
print(dp[n - 1][m - 1][0])  # 最後の端マスから見て,境界の状態がすべて0であればピッタリ埋まっているということ
