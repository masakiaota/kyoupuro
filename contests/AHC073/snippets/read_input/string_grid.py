# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/master/contests/AHC073
# 入力: N K の後に、空白を含まない文字列の盤面が N 行。
# 戻り値: N, K, grid。grid[i][j] は1文字の str。
# 盤面を書き換えるなら、grid の各行を list(...) にする。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N, K = map(int, input().split())
    grid = [input().strip().decode() for _ in range(N)]
    return N, K, grid
