# P8の問題をN<1000にした場合
# O(N^2 log N)
from random import randint
from bisect import bisect_left


def index(a, x):
    'Locate the leftmost value exactly equal to x'
    return -1


n = 1000
m = 10 ** 5  # ちょうどなってほしい数(ここでは決め打ち)
k = [randint(1, 2 * 10 ** 6) for _ in range(n)]

# 事前に、2つ選んだときにいくつになるか列挙しておく
candi = set()
for i in range(n):
    for j in range(i, n):
        candi.add(k[i] + k[j])
candi = sorted(candi)

# a+b、c+dの組み合わせで足すと=mになるのを二分探索で探す
for ab in candi:
    x = m - ab  # イコールになっててほしいやつ
    i = bisect_left(candi, x)
    if i != len(candi) and candi[i] == x:
        print('Yes')
        break
else:
    print('No')
