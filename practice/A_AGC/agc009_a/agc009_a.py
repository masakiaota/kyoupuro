# https://atcoder.jp/contests/agc009/tasks/agc009_a
# 普通にBの倍数かつAより大きいものの中で最小の数を高速に求めれば良い
# いや[0,i)を+1増やすらしい。めちゃくちゃめんどくさいやつだ

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


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


N = a_int()
A, B = read_col(N)
# 後ろから決定していく
n_pushed = 0  # すでに何回押されたか(a+n_pushedがその時点での数字の大きさに成る)
for a, b in zip(reversed(A), reversed(B)):
    a += n_pushed
    # x = (ceil(a / b)) * b
    x = ((a - 1) // b + 1) * b  # 整数のceilはこうかける！
    # print(x)
    n_pushed += x - a

print(n_pushed)
