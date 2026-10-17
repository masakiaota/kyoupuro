# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
# 入力: 整数の列が1行。件数を表す先頭行はない。
# 戻り値: values。初期入力がこの1行だけの対話型にも使える。


def read_input():
    import sys

    values = list(map(int, sys.stdin.buffer.readline().split()))
    return values
