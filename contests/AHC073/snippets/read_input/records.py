# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/9afb105b7761edfd67f158672e5c47caa90510f8/contests/AHC073
# 入力: N の後に、整数を空白で区切った行が N 行。
# 戻り値: N, rows。後続の関数では N, rows = data と受け取る。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N = int(input())
    rows = [list(map(int, input().split())) for _ in range(N)]
    return N, rows
