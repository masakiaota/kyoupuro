# https://atcoder.jp/contests/agc011/tasks/agc011_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


N, C, K = read_ints()
T, = read_col(N)
T.sort()

# 条件が2つのgreedy
# つまりsorted(T)に対して、C人乗ったら出発。K分経ったら出発を繰り返す。
bus = []  # バスの乗客
ans = 0  # なんかいバスが出発したか
for t in T:
    if (len(bus) and bus[0] + K < t) or len(bus) == C:  # これ以上待てないor C人になった
        ans += 1  # バスを出発させる
        # print(bus)
        bus = []
    bus.append(t)
if bus:
    ans += 1
print(ans)
