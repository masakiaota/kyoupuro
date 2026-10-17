# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 入力: 件数を表す行がなく、整数の行が固定数だけ続く。
# 変える箇所: ROWS。文字盤面なら読み込みを input().strip().decode() にする。
# 戻り値: rows。後続の関数では rows = data と受け取る。


def read_input():
    import sys

    input = sys.stdin.buffer.readline
    ROWS = 1000
    rows = [list(map(int, input().split())) for _ in range(ROWS)]
    return rows
