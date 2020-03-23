# https://atcoder.jp/contests/abc047/tasks/arc063_b
#
# 高橋くんが利益を上げる状況は街iで安く仕入れてi+jで高く売ること
# min と maxの差が最大になる点がいくつ存在するかをカウントすれば良い
#


import sys
from collections import Counter
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, T = read_ints()
A = read_ints()


# minの点を覚えておく
# 各点に足して差を出す(その点を使うと達成可能な利益)
# minはちゃんと更新
scores = []
mi = 10**9 + 7
for a in A:
    mi = min(mi, a)
    scores.append(a - mi)

ans = Counter(scores)
print(ans[max(scores)])
