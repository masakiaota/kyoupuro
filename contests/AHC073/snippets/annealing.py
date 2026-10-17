# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/9afb105b7761edfd67f158672e5c47caa90510f8/contests/AHC073
"""
焼きなまし法の分かりやすい全再評価版。このファイル全体を main.py にコピーして使う。

編集する関数一覧: read_input, make_initial, score, neighbor, write_output
大きいほど良い評価値。最小化は -cost を返す。
調整する数値: TIME_LIMIT, START_TEMPERATURE, END_TEMPERATURE, RANDOM_SEED
AHC073の問題仕様は未記入。

TIME_LIMIT は公式の時間制限より余裕を持たせて変更する。
START_TIME は実行開始直後に記録し、初期化と入力の時間も含む。
neighbor の1回が重いと、その実行中は時間を確認できない。
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


# 引数: 入力 data。戻り値: 完成解 state。data は書き換えない。
# 記入例（AHC073とは無関係）:
# n, _ = data
# return list(range(n))
def make_initial(data):
    raise NotImplementedError("TODO: make_initial 最初の完成解を作る")


# 引数: 入力 data と完成解 state。戻り値: 大きいほど良い数値。どちらも書き換えない。
# 記入例（AHC073とは無関係）:
# _, values = data
# cost = sum(abs(values[state[i]] - values[state[i - 1]]) for i in range(1, len(state)))
# return -cost
def score(data, state):
    raise NotImplementedError("TODO: score 完成解の評価値を返す")


# 引数: data, 現在解 state, 乱数 rng。戻り値: state と独立した候補解。state/data は書き換えない。
# 記入例（AHC073とは無関係）:
# candidate = state[:]  # 入れ子のリストなら [row[:] for row in state]
# i, j = rng.sample(range(len(state)), 2)
# candidate[i], candidate[j] = candidate[j], candidate[i]; return candidate
def neighbor(data, state, rng):
    raise NotImplementedError("TODO: neighbor 元の state を変更せず、新しい候補解を返す")


# 引数: 最良の完成解。戻り値: なし。answer は書き換えない。
# 記入例（AHC073とは無関係）:
# print(len(answer))
# print(*answer)
def write_output(answer):
    raise NotImplementedError("TODO: write_output answer を指定形式で出力する")


def anneal(data):
    rng = random.Random(RANDOM_SEED)
    current_state = make_initial(data)
    current_score = score(data, current_state)
    best_state = current_state
    best_score = current_score
    iterations = 0
    end_time = START_TIME + TIME_LIMIT

    while True:
        now = time.perf_counter()
        if now >= end_time:
            break
        progress = min((now - START_TIME) / TIME_LIMIT, 1.0)
        temperature = START_TEMPERATURE * (END_TEMPERATURE / START_TEMPERATURE) ** progress

        candidate = neighbor(data, current_state, rng)
        candidate_score = score(data, candidate)
        delta = candidate_score - current_score
        iterations += 1

        if delta >= 0 or rng.random() < math.exp(delta / temperature):
            current_state = candidate
            current_score = candidate_score
            if current_score > best_score:
                best_state = current_state
                best_score = current_score

    print(f"iterations={iterations} score={best_score}", file=sys.stderr)
    return best_state


def main():
    data = read_input()
    answer = anneal(data)
    write_output(answer)


if __name__ == "__main__":
    main()
