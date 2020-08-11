import sys
read = sys.stdin.readline
ra = range
enu = enumerate


def a_int(): return int(read())


from itertools import product


# 画像でおいた自分の解説見て
N = a_int()
B = []
for _ in ra(N):
    A = read()[:-1]  # floatで扱うと誤差で死ぬ
    if '.' not in A:
        A += '.'
    for _ in range(9 - A[::-1].index('.')):
        A += '0'
    B.append(int(A[:-10] + A[-9:]))

N = []
M = []
for b in B:
    n = 0
    while ~b & 1:  # 偶数である限り
        b >>= 1
        n += 1
    N.append(n)

    m = 0
    while b % 5 == 0:  # 5で割れる限り
        b //= 5
        m += 1
    M.append(m)

cnt = [[0] * 19 for _ in ra(19)]
ans = 0
for n, m in zip(N, M):
    x = max(18 - n, 0)
    y = max(18 - m, 0)
    for i, j in product(range(x, 19), range(y, 19)):
        ans += cnt[i][j]
    # このデータをcntに追加
    cnt[min(n, 18)][min(m, 18)] += 1

print(ans)
