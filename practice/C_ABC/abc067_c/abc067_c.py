# https://atcoder.jp/contests/abc067/tasks/arc078_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


INF = 10 ** 15

N = a_int()
A = ints()

# 山札の上から取るから位置を全探索可能
x = 0  # すぬけくんのカードの合計
y = sum(A)
ans = INF
for a in A[:-1]:
    x += a
    y -= a
    ans = min(ans, abs(x - y))
print(ans)
