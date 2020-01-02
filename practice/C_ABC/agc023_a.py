# https://atcoder.jp/contests/agc023/tasks/agc023_a
# 累積和とってからcounterすれば終わりでは？


N = int(input())
A = list(map(int, input().split()))

from itertools import accumulate
from collections import Counter

A_acc_cnt = Counter(accumulate(A))
ans = 0
for a, c in A_acc_cnt.items():
    if a == 0:
        c += 1
    if c == 1:
        continue
    ans += c * (c - 1) // 2

print(ans)
