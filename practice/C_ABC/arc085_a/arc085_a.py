# https://atcoder.jp/contests/abc078/tasks/arc085_a

N, M = map(int, input().split())


time_trial = 1900 * M + (N - M) * 100

# 1/2**M の確率で打ち切るときの打ち切りまでの平均回数は？
# 無限級数を無理やり数値的に出しちゃうのは？
# mul = 0
# for i in range(1, 100000):
#     mul += i * (1 - ((1 / 2) ** M))**(i - 1) * ((1 / 2) ** M)
# # print(mul)
# from math import ceil
# print(ceil(time_trial * mul))


# 重要な性質→trialを確率pで打ち切るとき、期待されるtrial数は？→1/p
# 1/1-xのマクローリン展開をするとΣの項の形と一致可能。式を整理すると1/pになる
print(time_trial * 2**M)
