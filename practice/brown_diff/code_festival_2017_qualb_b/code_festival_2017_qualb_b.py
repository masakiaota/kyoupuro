# https://atcoder.jp/contests/code-festival-2017-qualb/tasks/code_festival_2017_qualb_b
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(text: str):
    print(text)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


from collections import Counter
N = a_int()
D = ints()
M = a_int()
T = ints()

cnt = Counter(D)
for t in T:
    cnt[t] -= 1
    if cnt[t] < 0:
        exit('NO')

print('YES')
