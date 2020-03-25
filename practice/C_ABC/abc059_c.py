# https://atcoder.jp/contests/abc059/tasks/arc072_a
# 累積和を作っておいて、その累積和の符号が交互になるようにしていく
# 累積和のi以降の要素をすべて変更する代わりにいくつ足せばいいのか持っておく(imos法のイメージ)

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


class cumsum1d:  # 一次元累積和クラス
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]

    def __call__(self, i):
        # i番目までの合計
        return self.ls_accum[i + 1]


N = read_a_int()
A = read_ints()

A = cumsum1d(A)
# ただし第一項が0とか途中0になる場合がめんどくさい
# →二通りやって少ないほうでいいじゃん
ans1 = 0
bias = 0
pre_sign = 1  # 第一項が負と仮定
for i in range(N):
    now = A(i) + bias
    if pre_sign * now < 0:  # 異符号
        pass
    elif pre_sign * now >= 0:  # 同符号
        ans1 += abs(now) + 1
        bias += -pre_sign * (abs(now) + 1)
    pre_sign = 1 if pre_sign == -1 else -1

ans2 = 0
bias = 0
pre_sign = -1  # 第一項が正と仮定
for i in range(N):
    now = A(i) + bias
    if pre_sign * now < 0:  # 異符号
        pass
    elif pre_sign * now >= 0:  # 同符号
        ans2 += abs(now) + 1
        bias += -pre_sign * (abs(now) + 1)
    pre_sign = 1 if pre_sign == -1 else -1
print(min(ans1, ans2))
