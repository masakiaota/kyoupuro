# https://atcoder.jp/contests/abc081/tasks/arc086_b
# 全部負か全部正なら簡単。じゃその状態に持っていけないか考えるべきだよね？


import sys
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
ma, mi = max(A), min(A)
print(2 * N - 2)
if abs(ma) > abs(mi):  # すべて正にするのが良い
    x = A.index(ma)
    for y in range(N):
        if x == y:
            continue
        print(x + 1, y + 1)
    # 累積和みたいにすれば良い
    for i in range(1, N):
        print(i, i + 1)
else:  # すべて負にするのがよい
    x = A.index(mi)
    for y in range(N):
        if x == y:
            continue
        print(x + 1, y + 1)
    for i in range(N, 1, -1):
        print(i, i - 1)
