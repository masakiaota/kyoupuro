# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/master/contests/AHC073
"""
対話型問題用の行単位入出力骨組み。このファイル全体を main.py にコピーして使う。

編集する関数一覧: read_input, read_turn, choose_action
大きいほど良い評価値。最小化は -cost を返す（この骨組みでは評価値は未使用）。
最初に選ぶ設定: SEND_FIRST = True なら自分が先に出力、False なら先に相手の入力を読む。
出力が複数行なら write_output も編集する。
調整する数値: なし
AHC073の問題仕様は未記入。

sys.stdin.buffer.read() は使わず、必ず input() で1行ずつ読む。
終了通知を受けたとき read_turn が None を返すよう編集する。

SEND_FIRST = True の順番: 初期入力 -> choose_action(data, None) -> 出力 -> 相手の入力 -> ...
SEND_FIRST = False の順番: 初期入力 -> 相手の入力 -> choose_action(data, turn) -> 出力 -> ...
"""

SEND_FIRST = True


# 引数: なし。戻り値: 初期入力と自分の状態。受け取った状態: なし。
# 記入例（AHC073とは無関係）:
# n = int(input())
# values = list(map(int, input().split()))
# return {"n": n, "values": values, "turn": 0}
# コピー元: snippets/read_input/README.md。選んだ read_input() 全体と置き換える。
def read_input():
    raise NotImplementedError("TODO: read_input input() で初期入力を1行ずつ読む")


# 引数: 自分の状態 data。戻り値: 相手の応答。終了時は None。data は書き換えない。
# 記入例（AHC073とは無関係）:
# line = input()
# if line == "-1": return None
# return list(map(int, line.split()))
def read_turn(data):
    raise NotImplementedError("TODO: read_turn input() で1行読み、終了通知なら None を返す")


# 引数: 書き換え可能な data と相手の応答 turn。送信先行の初回だけ turn は None。戻り値: 次の出力。
# 記入例（AHC073とは無関係）:
# if turn is not None: data["last_reply"] = turn
# data["turn"] += 1
# choice = min(range(data["n"]), key=lambda i: data["values"][i])
# return [choice]
def choose_action(data, turn):
    raise NotImplementedError("TODO: choose_action 次の行動を決め、必要なら data も更新する")


def write_output(answer):
    if isinstance(answer, (list, tuple)):
        print(*answer, flush=True)
    else:
        print(answer, flush=True)


def main():
    data = read_input()
    if SEND_FIRST:
        # 初回の None は「まだ相手の入力がない」という意味で、終了通知ではない。
        answer = choose_action(data, None)
        write_output(answer)
    while True:
        turn = read_turn(data)
        if turn is None:
            break  # read_turn が返す None は終了通知だけを表す。
        answer = choose_action(data, turn)
        write_output(answer)  # flush=True で即時送信する。


if __name__ == "__main__":
    main()
