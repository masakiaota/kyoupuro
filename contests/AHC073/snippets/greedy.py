# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/master/contests/AHC073
"""
候補の中から毎回一番良いものを選ぶ貪欲法。このファイル全体を main.py にコピーして使う。

編集する関数一覧: read_input, make_initial, num_steps, expand, priority, write_output
大きいほど良い評価値。最小化は -cost を返す。
調整する数値: なし
AHC073の問題仕様は未記入。

expand で次の候補を列挙し、priority でその良さを決めれば、選択ループはそのまま使える。
"""


# 引数: なし。戻り値: 入力全体をまとめた値。受け取った状態: なし。
# 記入例（AHC073とは無関係）:
# n = int(input())
# values = list(map(int, input().split()))
# return n, values
# コピー元: snippets/read_input/README.md。選んだ read_input() 全体と置き換える。
def read_input():
    raise NotImplementedError("TODO: read_input 入力を読み、他の関数に渡す値を返す")


# 引数: 入力 data。戻り値: 深さ0の未完成状態。data は書き換えない。
# 記入例（AHC073とは無関係）:
# return []
# # 以後の expand で1つずつ選択を追加する
def make_initial(data):
    raise NotImplementedError("TODO: make_initial 深さ0の未完成状態を返す")


# 引数: 入力 data。戻り値: 選択を繰り返す回数。data は書き換えない。
# 記入例（AHC073とは無関係）:
# n, _ = data
# return n
def num_steps(data):
    raise NotImplementedError("TODO: num_steps 完成までに何回選ぶかを返す")


# 引数: data, 未完成状態 state, 現在の深さ step。戻り値: 独立した次状態の反復可能な値。元は書き換えない。
# 記入例（AHC073とは無関係）:
# for choice in range(data[0]):
#     if choice not in state:
#         yield state + [choice]
def expand(data, state, step):
    raise NotImplementedError("TODO: expand state の次にあり得る状態を列挙する")


# 引数: data, expand が作った next_state, 選択前の深さ step。戻り値: 次状態の優先度。元は書き換えない。
# 記入例（AHC073とは無関係）:
# _, values = data
# chosen = next_state[-1]
# return values[chosen]
def priority(data, next_state, step):
    raise NotImplementedError("TODO: priority 今回の候補の良さを返す")


def greedy(data):
    state = make_initial(data)
    for step in range(num_steps(data)):
        next_state = max(
            expand(data, state, step),
            key=lambda candidate: priority(data, candidate, step),
            default=None,
        )
        if next_state is None:
            raise RuntimeError(
                f"step={step} の候補が0件。expand() が少なくとも1つ候補を返すよう修正する"
            )
        state = next_state
    return state


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
