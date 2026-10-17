# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/master/contests/AHC073
"""
貪欲法の最小骨組み。問題に合わせてこの main.py を直接編集する。

編集する関数一覧: read_input, greedy, write_output
大きいほど良い評価値。最小化は -cost を返す（この骨組みでは評価値は未使用）。
調整する数値: なし
AHC073の問題仕様は未記入。
"""


# 引数: なし。戻り値: 入力全体をまとめた値。受け取った状態: なし。
# 記入例（AHC073とは無関係）:
# n = int(input())
# values = list(map(int, input().split()))
# return n, values
# コピー元: snippets/read_input/README.md。選んだ read_input() 全体と置き換える。
def read_input():
    raise NotImplementedError("TODO: read_input 入力を読み、greedy に渡す値を返す")


# 引数: read_input の戻り値。戻り値: 完成した解。data は書き換えない。
# 記入例（AHC073とは無関係）:
# n, values = data
# order = sorted(range(n), key=lambda i: -values[i])
# return order
def greedy(data):
    raise NotImplementedError("TODO: greedy 先頭から順に選び、完成した解を返す")


# 引数: greedy の完成解。戻り値: なし。answer は書き換えない。
# 記入例（AHC073とは無関係）:
# print(len(answer))
# print(*answer)
def write_output(answer):
    raise NotImplementedError("TODO: write_output answer を指定形式で出力する")


def main():
    data = read_input()
    answer = greedy(data)
    write_output(answer)


if __name__ == "__main__":
    main()
