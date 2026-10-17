# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 対話型の初期入力: N T sigma、その後に整数ペアが N 行。
# 戻り値: N, T, sigma, items。sigma も整数の例。
# この関数は初期入力だけを読む。各手番の応答は read_turn に書く。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N, T, sigma = map(int, input().split())
    items = [tuple(map(int, input().split())) for _ in range(N)]
    return N, T, sigma, items
