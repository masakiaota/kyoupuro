import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
H = read_ints()

# https://atcoder.jp/contests/dp/tasks/dp_a
# 全探索ではナンセンス(というより処理が終わらない)
# i+1 or i+2のうち小さい方を選びつづければいいということにすぐに気がつく
# こういったように、今の状態から次の状態が決まるときはDPが使えることがおおい(この問題はgreedy気味にも捉えることができるが。)
# 途中までの状態が決まれば、最善手を選ぶことで次の状態が決定できるのを実装する。
# 今回の場合だったらhまでの最善の状態というのはmin(2つ前までの最善の状態+差, 1つ前までの最善の状態+差)なのでこれでdpテーブルを埋めていくことができる。
# こういった最小化問題はinfで初期化すると良い

import numpy as np
dp = np.full((N), float('inf'))
dp[0] = 0
dp[1] = abs(H[0] - H[1])

# この概念はいわゆる貰うDP
# for i in range(2, N):
#     dp[i] = min(dp[i - 1] + abs(H[i - 1] - H[i]),
#                 dp[i - 2] + abs(H[i] - H[i - 2]))

# print(dp[-1])
# 配るDPで実装してみる
for i in range(0, N-2):
    dp[i + 1] = min(dp[i] + abs(H[i] - H[i+1]), dp[i+1])
    dp[i + 2] = min(dp[i] + abs(H[i] - H[i + 2]), dp[i + 2])

# これだと最後のdp[N-1]がdp[N-2]から値を受け取れないので最後につじつま合わせ

dp[-1] = min(dp[-2] + abs(H[-2] - H[-1]), dp[-1])

print(int(dp[-1]))
