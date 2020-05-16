# 蟻本にdpの解説がないので他ブログを参考
# https://lvs7k.github.io/posts/2018/11/pccb-easy-7/

# 確率DPと言われるやつかもしれない。独立性と排反事象から次の状態を作れないか考えるのが良さそう(?)

from itertools import product


def solve(M, P, X):
    '''
    dp[r][g] ... 最善戦略におけるクリアできる確率。最初の所持金がグループgに属するとき、残りrラウンドある場合。
    更新則
    dp[r + 1][g] = max_j (P * dp[r][g + j] + (1 - P) * dp[r][g - j])
    ∵ 残りr+1ラウンドで任意の金額を掛けたときに、勝つか負けるかで所持金が変化するのでg + j,g - jから遷移が来ることがわかる。
    r+1ラウンドで勝って結果的にクリアできる確率はP * dp[r][g + j] で
    r+1ラウンドで負けて結果的にクリアできる確率は(1 - P) * dp[r][g - j]である。
    よってr+1ラウンド目で結果的にクリアできる確率はその2つの和になる。
    賭ける金額によって確率が変動するので、最善戦略になるようにjについてmaxを取る。
    初期条件
    dp[0][-1]=1 ∵ 最初から10^6以上であれば賞金を受け取れる。
    '''

    n = pow(2, M) + 1  # 所持金のグループ数
    dp = [[0.0] * (n) for _ in range(M + 1)]
    dp[0][-1] = 1.0

    for r, g in product(range(M), range(n)):
        jub = min(g, n - g - 1)
        for j in range(jub + 1):
            dp[r + 1][g] = max(dp[r + 1][g],
                               P * dp[r][g + j] + (1 - P) * dp[r][g - j])

    print(dp[M][X * (n - 1) // (10**6)])
    print(*dp, sep='\n')


# 入力例1
M = 1
P = 0.5
X = 500000
solve(M, P, X)

# 入力例1(original)
M = 2
P = 0.5
X = 500000
solve(M, P, X)
# 入力例2
M = 3
P = 0.75
X = 600000
solve(M, P, X)
