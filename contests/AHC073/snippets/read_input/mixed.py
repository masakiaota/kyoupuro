# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 入力: N M eps（整数、整数、小数）、続く M 行は d x0 y0 ... x(d-1) y(d-1)。
# 戻り値: N, M, eps, shapes。shapes[k] は座標ペアのリスト。
# 初期入力の M 行だけを読むので、対話型の初期入力にも使える。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    N, M, eps = input().split()
    N, M, eps = int(N), int(M), float(eps)
    shapes = []
    for _ in range(M):
        row = list(map(int, input().split()))
        d = row[0]
        cells = [(row[1 + 2 * j], row[2 + 2 * j]) for j in range(d)]
        shapes.append(cells)
    return N, M, eps, shapes
