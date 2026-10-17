# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/f6ab34c0de0880a491742d528bb99a58714f5063/contests/AHC073
"""
焼きなまし法の高速な差分評価版。このファイル全体を main.py にコピーして使う。

編集する関数一覧: read_input, make_initial, score, apply_random_change, undo_change, write_output
大きいほど良い評価値。最小化は -cost を返す。
調整する数値: TIME_LIMIT, START_TEMPERATURE, END_TEMPERATURE, RANDOM_SEED
AHC073の問題仕様は未記入。
状態が入れ子のリストなら copy_state だけを変更する。

TIME_LIMIT は公式の時間制限より余裕を持たせて変更する。
START_TIME は実行開始直後に記録し、初期化と入力の時間も含む。
apply_random_change の1回が重いと、その実行中は時間を確認できない。
"""

import time

START_TIME = time.perf_counter()

import math
import random
import sys

TIME_LIMIT = 1.8
# 温度は1変更の得点差が目安。高いほど悪化案も試す。開始と終了は正の値にする。
START_TEMPERATURE = 100.0
END_TEMPERATURE = 1.0
RANDOM_SEED = 0


# 引数: なし。戻り値: 入力全体をまとめた値。受け取った状態: なし。
# 記入例（AHC073とは無関係）:
# n = int(input())
# values = list(map(int, input().split()))
# return n, values
# コピー元: snippets/read_input/README.md。選んだ read_input() 全体と置き換える。
def read_input():
    raise NotImplementedError("TODO: read_input 入力を読み、他の関数に渡す値を返す")


# 引数: 入力 data。戻り値: リストで表した完成解。data は書き換えない。
# 記入例（AHC073とは無関係）:
# n, _ = data
# return [0] * n
def make_initial(data):
    raise NotImplementedError("TODO: make_initial 最初の完成解を作る")


# 引数: リストで表した state。戻り値: 最良解保存用のコピー。state は書き換えない。
# 通常のリストは state[:] でよい。
# 入れ子のリストなら return [row[:] for row in state] にここだけ変更する。
def copy_state(state):
    return state[:]


# 引数: 入力 data と完成解 state。戻り値: 大きいほど良い数値。どちらも書き換えない。
# 記入例（AHC073とは無関係）:
# return sum(state)
# # 最小化なら return -sum(state)
def score(data, state):
    raise NotImplementedError("TODO: score 初期解の評価値を一度だけ計算する")


# 引数: data, 書き換える state, 乱数 rng。戻り値: (評価値の増分, 取消し用情報)。
# 記入例（AHC073とは無関係）:
# i = rng.randrange(len(state)); old = state[i]; new = 1 - old
# state[i] = new
# return new - old, (i, old)
def apply_random_change(data, state, rng):
    raise NotImplementedError("TODO: apply_random_change state を1か所変更し、評価増分と取消し情報を返す")


# 引数: data, 書き換える state, apply_random_change の undo_info。戻り値: なし。
# 記入例（AHC073とは無関係）:
# i, old = undo_info
# state[i] = old
def undo_change(data, state, undo_info):
    raise NotImplementedError("TODO: undo_change 直前の変更を完全に取り消す")


# 引数: 最良の完成解。戻り値: なし。answer は書き換えない。
# 記入例（AHC073とは無関係）:
# print(len(answer))
# print(*answer)
def write_output(answer):
    raise NotImplementedError("TODO: write_output answer を指定形式で出力する")


def anneal(data):
    rng = random.Random(RANDOM_SEED)
    state = make_initial(data)
    current_score = score(data, state)
    best_state = copy_state(state)
    best_score = current_score
    iterations = 0
    end_time = START_TIME + TIME_LIMIT

    while True:
        now = time.perf_counter()
        if now >= end_time:
            break
        progress = min((now - START_TIME) / TIME_LIMIT, 1.0)
        temperature = START_TEMPERATURE * (END_TEMPERATURE / START_TEMPERATURE) ** progress

        delta, undo_info = apply_random_change(data, state, rng)
        iterations += 1
        if delta >= 0 or rng.random() < math.exp(delta / temperature):
            current_score += delta
            if current_score > best_score:
                best_state = copy_state(state)
                best_score = current_score
        else:
            undo_change(data, state, undo_info)

    print(f"iterations={iterations} score={best_score}", file=sys.stderr)
    return best_state


def main():
    data = read_input()
    answer = anneal(data)
    write_output(answer)


if __name__ == "__main__":
    main()
