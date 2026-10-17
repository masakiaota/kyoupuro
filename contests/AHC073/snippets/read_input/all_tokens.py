# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 通常型専用。入力全体を一括で読むため、対話型には使わない。
# 入力: N、その後に x y r が N 組。改行の位置には依存しない。
# 戻り値: N, rows。列を変えるときは next(tokens) の個数と変換型を変える。


def read_input():
    import sys

    tokens = iter(sys.stdin.buffer.read().split())
    N = int(next(tokens))
    rows = [
        (int(next(tokens)), int(next(tokens)), int(next(tokens)))
        for _ in range(N)
    ]
    return N, rows
