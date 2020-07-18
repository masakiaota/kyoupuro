# https://atcoder.jp/contests/arc010/tasks/arc010_2
# 月日を連番に直したいね
md_to_i = {}
i = 0
for m in range(1, 13):
    D = 32
    if m in (4, 6, 9, 11):
        D = 31
    if m == 2:
        D = 30
    for d in range(1, D):
        md_to_i[str(m) + '/' + str(d)] = i
        i += 1

is_holiday = [False] * 367
for d in range(366):
    if d % 7 == 0 or (d + 1) % 7 == 0:
        is_holiday[d] = True

N = int(input())
for _ in range(N):
    idx = md_to_i[input()]
    while idx < 367 and is_holiday[idx] == True:
        idx += 1
    is_holiday[idx] = True
    is_holiday[366] = False  # 次の年に繰越だけど(これは2012年の最大連休にはならない)

from itertools import groupby
ans = 0
for k, v in groupby(is_holiday[:-1]):
    if k:
        ans = max(ans, len(list(v)))
print(ans)
