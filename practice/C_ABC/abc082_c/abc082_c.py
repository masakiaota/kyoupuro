# https://atcoder.jp/contests/abc082/tasks/arc087_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


from collections import Counter
N = a_int()
A = ints()
# Counterして成り立たない数字を取り除いた数が答え

cnt = Counter(A)
ans = 0
for k, v in cnt.items():
    if k < v:
        ans += v - k
    if k > v:
        ans += v
print(ans)
