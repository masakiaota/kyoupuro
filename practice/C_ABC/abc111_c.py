# https://atcoder.jp/contests/abc111/tasks/arc103_a
# 1個飛ばしで列をカウントする
# それぞれについてもっとも多く数列に現れる数字の出現回数の合計cに対してlen(A)-cが答え
# cは具体的には、2つのカウントに対して同じ数字だったら合計しない。で次に数の多い方を採用


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


n = read_a_int()
V = read_ints()

from collections import Counter
gu = Counter(V[0::2]).most_common()
ki = Counter(V[1::2]).most_common()

c = gu[0][1] + ki[0][1]
if gu[0][0] == ki[0][0]:
    c //= 2
    if len(gu) == 1 and len(ki) == 1:
        pass
    elif len(gu) == 1:
        c += ki[1][1]
    elif len(ki) == 1:
        c += ki[1][1]
    else:
        c += max(gu[1][1], ki[1][1])

print(n - c)
