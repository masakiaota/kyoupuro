# https://atcoder.jp/contests/agc011/tasks/agc011_b

# 色は最後に食ったcreatureになる
# 食って自分の色を残したまま多くなる→小さすぎる生物は食われるしか無い(その色は失われる)
# ソートして、連結していって、連結できなくなった部分はその色に成ることはできない

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
A.sort()
no_idx = 0  # 初めてその色になれないidx [0,no_idx)の色を取ることはできない
s = 0
# いわゆる累積和でどうにかなる
for i, a in enu(A):
    if s * 2 < a:
        no_idx = i
    s += a

print(N - no_idx)
