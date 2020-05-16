# これむずい もう一回やらないとわからなさそう

'''
dp[i][j] ... (A[i],A[j])の区間に含まれる囚人を開放するのに必要な金貨の最小枚数
更新則
dp[i][j] = min(dp[i][k]+dp[k][j]) + A[j] - A[i] - 2
for j-i(=w)を2から(Q+1)まで広げつつ更新する
∵ 区間(A[i],A[j])の最小枚数は、抜く囚人をkとすると、
= dp[i][k]+dp[k][j] (P122図の①と②)
+A[j]-A[i] - 2 (区間の人数(-2はiの位置の分とkの位置の分))
となる。この中で最小値を探すってわけ。

初期条件
dp[q][q+1]=0 for q=[0,Q+1) ∵この範囲の中に取り出すべき囚人はいない(取引もないので金貨も必要ない)
'''


def solve(P, Q, A):
    A = [0] + A + [P + 1]
    dp = [[float('inf')] * (Q + 2) for _ in range(Q + 1)]
    # 初期化
    for q in range(Q + 1):
        dp[q][q + 1] = 0  # 隣の開放する囚人との間は誰も開放しないので金貨は0枚

    for w in range(2, Q + 2):  # 区間を広げていく
        for i in range(Q):
            j = i + w
            if j > Q + 1:
                break  # idxが範囲外で終了
            for k in range(i + 1, j):
                dp[i][j] = min(dp[i][j],
                               dp[i][k] + dp[k][j] + A[j] - A[i] - 2)

    print(*dp, sep='\n')
    print(dp[0][Q + 1])


# 入力例1
P = 8
Q = 1
A = [3]
solve(P, Q, A)

# 入力例2
P = 20
Q = 3
A = [3, 6, 14]
solve(P, Q, A)
