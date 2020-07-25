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


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right, insort_left, insort_right
from functools import reduce

N = a_int()
XYP = read_tuple(N)
X, Y, P = zip(*XYP)
# i=0,1のときは簡単
# i=>2のときにどうするか...
# まてよ、各点について、[なし、縦、横]の三種類の全探索なら3^15=14348907でぎり間に合う
# それで何本鉄道路線を敷いたかのdictに突っ込んであげれば良い
# なかのループも考えると10^9ぐらいになってCppじゃないと間に合わんなぁ

ans = defaultdict(lambda: 10**15 + 1)

for types in product(range(3), repeat=N):
    n_rails = 0
    Xs = []  # 縦のrails #単調増加
    Ys = []  # 縦のrails #単調増加
    i_walk = []  # 歩くのが必要な集落のidx
    for i, ty in enu(types):
        if ty == 0:  # railsをしかない
            i_walk.append(i)
        elif ty == 1:  # railsを縦に敷く
            Xs.append(X[i])
            n_rails += 1
        else:  # railを横にしく
            Ys.append(Y[i])
            n_rails += 1
    Xs.append(10**9)
    Ys.append(10**9)
    insort_left(Xs, 0)
    insort_left(Ys, 0)
    # 歩くコストの計算
    cost = 0
    for i in i_walk:
        # Xの一番路線
        idx = bisect_left(Xs, X[i])
        d_x = min(abs(Xs[idx] - X[i]), abs(Xs[idx - 1] - X[i]))
        # Yの一番近い路線
        idx = bisect_left(Ys, Y[i])
        # print(idx, Ys, Y)
        d_y = min(abs(Ys[idx] - Y[i]), abs(Ys[idx - 1] - Y[i]))
        cost += min(d_x, d_y) * P[i]

    ans[n_rails] = min(ans[n_rails], cost)

for i in range(N + 1):
    print(ans[i])
