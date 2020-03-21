# https://atcoder.jp/contests/abc019/tasks/abc019_3
# 限界まで2の階乗で割りまくる

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
tmp = []


def ret_origin(a):
    while a & 1 == 0:
        a //= 2
    return a


for a in A:
    tmp.append(ret_origin(a))
print(len(set(tmp)))
