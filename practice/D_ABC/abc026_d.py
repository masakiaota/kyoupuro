# https://atcoder.jp/contests/abc026/tasks/abc026_d
# ぱっと思いつくのはニュートンラプソン法
# でもどの解でも良いのなら二分探索でも良い気がする

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


A, B, C = read_ints()

from math import sin, pi


def F(t):
    return A * t + B * sin(pi * C * t)


def is_ok(t):
    # 100を超えるか
    return 100 <= F(t)


def meguru_bisect(ng, ok):
    '''
    連続値版めぐる二分探索
    '''
    while (abs(F(ok) - 100) > 10**-8):  # ここの誤差が重要やな
        mid = (ok + ng) / 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(0, 1000))
