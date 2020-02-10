N = int(input())
K = int(input())


def ketadp(X):
    X = '0' + str(X)
    dp = [[[0] * 4 for _ in range(2)] for _ in range(len(X))]
    dp[0][0][0] = 1
    for i in range(len(X) - 1):
        for j in range(2):
            for d in range(10 if j else int(X[i + 1]) + 1):
                if d == 0:
                    for k in range(4):
                        dp[i + 1][j or d < int(X[i + 1])][k] += dp[i][j][k]
                else:
                    for k in range(3):
                        dp[i + 1][j or d < int(X[i + 1])][k + 1] += dp[i][j][k]

    return dp[-1][0][K] + dp[-1][1][K]


print(ketadp(N))
