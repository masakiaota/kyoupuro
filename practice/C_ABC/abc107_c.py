# https://atcoder.jp/contests/abc107/tasks/arc101_a

# 累積和を使って区間ごとの時間を求めて、更にスタートからの時間を足す
# 考える区間は0から左にK個〜右K個までK個ずつずらす

import sys
from bisect import bisect_left
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
X = read_ints()


# すべて正の場合
if X[0] >= 0:
    print(X[K - 1])
    exit()
# 負の場合
if X[-1] <= 0:
    print(abs(X[-K]))
    exit()


zeroidx = bisect_left(X, 0)
if X[zeroidx] == 0:
    K -= 1

l = max(zeroidx - K, 0)
# r_max = min(zeroidx + K, len(X) - 1)

ans = 10**16

for i in range(K):
    ll = l + i
    rr = ll + K - 1
    if rr >= len(X):
        break
    # print(X[ll:ll + K])
    ans = min(ans, 2 * abs(X[ll]) + abs(X[rr]),
              abs(X[ll]) + 2 * abs(X[rr]))


print(ans)
