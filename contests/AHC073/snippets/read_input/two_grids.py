# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 入力: si sj、N 行の整数盤面 t、N 行の整数盤面 p。
# 変える箇所: N。サイズが先頭行に含まれるなら、そこから N も読む。
# 戻り値: si, sj, t, p。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N = 50
    si, sj = map(int, input().split())
    t = [list(map(int, input().split())) for _ in range(N)]
    p = [list(map(int, input().split())) for _ in range(N)]
    return si, sj, t, p
