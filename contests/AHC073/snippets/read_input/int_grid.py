# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/9afb105b7761edfd67f158672e5c47caa90510f8/contests/AHC073
# 入力: N M の後に、整数の盤面が N 行。
# 列数は各行の入力に従う。M は色数など、列数以外の値でもよい。
# 戻り値: N, M, grid。grid[i][j] は整数。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N, M = map(int, input().split())
    grid = [list(map(int, input().split())) for _ in range(N)]
    return N, M, grid
