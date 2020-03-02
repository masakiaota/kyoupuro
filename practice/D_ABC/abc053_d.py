# https://atcoder.jp/contests/abc053/tasks/arc068_b
# 具体的操作によって理解は深まる
# ポイントとなるのは、余った分はどういう操作をしても二枚づつ消すことができる。
# 余った分というのは、カードaに対してaの出現回数-1となる数字のことである。
# つまり余った分を単純に消せばよい。
# しかし余った分が奇数個のとき、余ってないカードから一枚犠牲にする必要がある。
# これらを実装すればおk

N = int(input())
A = list(map(int, input().split()))

from collections import Counter

A_cnt = Counter(A)
amari = sum([v - 1 for v in A_cnt.values()])
print(N - amari - 1 if amari & 1 else N - amari)
