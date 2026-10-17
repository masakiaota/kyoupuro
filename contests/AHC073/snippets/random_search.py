# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/master/contests/AHC073
"""
乱択で完成解を作り続け、最良解を返す。このファイル全体を main.py にコピーして使う。

編集する関数一覧: read_input, make_random, score, write_output
大きいほど良い評価値。最小化は -cost を返す。
調整する数値: TIME_LIMIT, RANDOM_SEED
AHC073の問題仕様は未記入。

TIME_LIMIT は公式の時間制限より余裕を持たせて変更する。
START_TIME は実行開始直後に記録し、初期化と入力の時間も含む。
make_random の1回が重いと、その実行中は時間を確認できない。
"""

import time

START_TIME = time.perf_counter()

import random
import sys

TIME_LIMIT = 1.8
RANDOM_SEED = 0


# 引数: なし。戻り値: 入力全体をまとめた値。受け取った状態: なし。
# 記入例（AHC073とは無関係）:
# n = int(input())
# values = list(map(int, input().split()))
# return n, values
# コピー元: snippets/read_input/README.md。選んだ read_input() 全体と置き換える。
def read_input():
    raise NotImplementedError("TODO: read_input 入力を読み、他の関数に渡す値を返す")


# 引数: 入力 data と乱数 rng。戻り値: 独立した完成解。data は書き換えない。
# 記入例（AHC073とは無関係）:
# n, _ = data
# answer = list(range(n))
# rng.shuffle(answer); return answer
def make_random(data, rng):
    raise NotImplementedError("TODO: make_random 乱数を使って新しい完成解を作る")


# 引数: 入力 data と完成解 state。戻り値: 大きいほど良い数値。どちらも書き換えない。
# 記入例（AHC073とは無関係）:
# _, values = data
# cost = sum(abs(values[state[i]] - values[state[i - 1]]) for i in range(1, len(state)))
# return -cost
def score(data, state):
    raise NotImplementedError("TODO: score 完成解の評価値を返す")


# 引数: 最良の完成解。戻り値: なし。answer は書き換えない。
# 記入例（AHC073とは無関係）:
# print(len(answer))
# print(*answer)
def write_output(answer):
    raise NotImplementedError("TODO: write_output answer を指定形式で出力する")


def search(data):
    rng = random.Random(RANDOM_SEED)
    best_state = make_random(data, rng)
    best_score = score(data, best_state)
    iterations = 1
    end_time = START_TIME + TIME_LIMIT

    while time.perf_counter() < end_time:
        candidate = make_random(data, rng)
        candidate_score = score(data, candidate)
        iterations += 1
        if candidate_score > best_score:
            best_state = candidate
            best_score = candidate_score

    print(f"iterations={iterations} score={best_score}", file=sys.stderr)
    return best_state


def main():
    data = read_input()
    answer = search(data)
    write_output(answer)


if __name__ == "__main__":
    main()
