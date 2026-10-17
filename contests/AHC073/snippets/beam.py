# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/9afb105b7761edfd67f158672e5c47caa90510f8/contests/AHC073
"""
問題で決めた手数まで完走するビームサーチ。このファイル全体を main.py にコピーして使う。

編集する関数一覧: read_input, make_initial, num_steps, expand, estimate, final_score, write_output
大きいほど良い評価値。最小化は -cost を返す。
調整する数値: BEAM_WIDTH
AHC073の問題仕様は未記入。

時間は BEAM_WIDTH で調整する。num_steps(data) の途中で打ち切らないため、未完成解を返さない。
"""

import heapq

BEAM_WIDTH = 100


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
# # 以後の expand で値を1つずつ追加する
def make_initial(data):
    raise NotImplementedError("TODO: make_initial 深さ0の未完成状態を返す")


# 引数: 入力 data。戻り値: 完成まで展開する手数。data は書き換えない。
# 記入例（AHC073とは無関係）:
# n, _ = data
# return n
def num_steps(data):
    raise NotImplementedError("TODO: num_steps 完成までに何回展開するかを返す")


# 引数: data, 未完成状態 state, 現在の深さ step。戻り値: 独立した次状態の反復可能な値。元は書き換えない。
# 記入例（AHC073とは無関係）:
# for choice in (0, 1):
#     child = state + [choice]
#     yield child
def expand(data, state, step):
    raise NotImplementedError("TODO: expand state の次にあり得る状態を列挙する")


# 引数: data, 未完成状態 state, stateの深さ step。戻り値: 完成時の見込み値。元は書き換えない。
# 記入例（AHC073とは無関係）:
# remaining = num_steps(data) - step
# return sum(state) + remaining
def estimate(data, state, step):
    raise NotImplementedError("TODO: estimate 未完成状態の見込み値を返す")


# 引数: data と num_steps(data) 回展開後の完成解。戻り値: 完成解の正式な評価値。元は書き換えない。
# 記入例（AHC073とは無関係）:
# return sum(state)
# # estimate と同じなら return estimate(data, state, num_steps(data))
def final_score(data, state):
    raise NotImplementedError("TODO: final_score 完成解の評価値を返す")


# 引数: 最良の完成解。戻り値: なし。answer は書き換えない。
# 記入例（AHC073とは無関係）:
# print(len(answer))
# print(*answer)
def write_output(answer):
    raise NotImplementedError("TODO: write_output answer を指定形式で出力する")


def beam_search(data):
    beam = [make_initial(data)]
    depth = num_steps(data)

    for step in range(depth):
        candidates = (
            (estimate(data, child, step + 1), child)
            for state in beam
            for child in expand(data, state, step)
        )
        selected = heapq.nlargest(BEAM_WIDTH, candidates, key=lambda item: item[0])
        if not selected:
            raise RuntimeError(
                "候補が0件。expand() が各 step で候補を返すよう修正し、BEAM_WIDTH > 0 も確認する"
            )
        beam = [state for _, state in selected]

    return max(beam, key=lambda state: final_score(data, state))


def main():
    data = read_input()
    answer = beam_search(data)
    write_output(answer)


if __name__ == "__main__":
    main()
