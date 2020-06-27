# https://atcoder.jp/contests/abc066/tasks/arc077_a

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


from collections import deque

n = a_int()
A = ints()
B = deque()
is_fliped = 0
for a in A:
    if is_fliped == 0:
        B.append(a)
    else:
        B.appendleft(a)
    is_fliped = 1 - is_fliped
if is_fliped:
    B.reverse()
print(*B)
