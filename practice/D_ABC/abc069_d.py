# https://atcoder.jp/contests/abc069/tasks/arc080_b
# 自明では？
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


H, W = read_ints()
N = read_a_int()
A = read_ints()

out = []
for i, a in enumerate(A, start=1):
    out.extend([i] * a)

# outをHWに並び替える #ただし連続性を担保するために蛇のようにする必要がある。

import numpy as np
out = np.array(out)
out = out.reshape((H, W))
for i, o in enumerate(out):
    if i & 1:
        print(*o)
    else:
        print(*o[::-1])
