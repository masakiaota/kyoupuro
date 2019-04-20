import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, input().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read() for _ in range(H)]


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


def main():
    N, K = read_ints()
    # W, V = read_col(N, 2)

    dp = [[-float('inf') for _ in range(K + 1)] for _ in range(N + 1)]

    dp[0] = [0] * (K + 1)
    for i in range(N + 1):
        dp[i][0] = 0

    for i in range(N):
        w, v = read_ints()
        for sum_w in range(K + 1):
            if sum_w - w < 0:
                dp[i + 1][sum_w] = dp[i][sum_w]
            else:
                dp[i + 1][sum_w] = max(
                    dp[i][sum_w],
                    dp[i][sum_w - w] + v)
    print(dp[-1][-1])


main()

# pythonでもpypyでもTLE
