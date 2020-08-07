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
from itertools import accumulate, product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b

# https://atcoder.jp/contests/abc059/tasks/arc072_a

# si=a1+..+aiと定義する
# s1...si,si+1,snの符号は+-+-... or -+-+...にしたい←両方試せばいい
# +-+-にするのに最小の操作は？


n = a_int()
A = ints()
S = list(accumulate(A))

#+-+-のとき
now = 1  # 正負
pad = 0
ans1 = 0
for s in S:
    s += pad
    if s * now <= 0:  # 異符号だったら修正する必要あり
        if now == 1:  # +にしたい場合
            n_ope = 1 - s
            pad += n_ope
        else:  # -にしたい場合
            n_ope = s + 1
            pad -= n_ope
        ans1 += n_ope
    now *= -1

#-+-+のとき
now = -1  # 正負
pad = 0
ans2 = 0
for s in S:
    s += pad
    if s * now <= 0:  # 異符号だったら修正する必要あり
        if now == 1:  # +にしたい場合
            n_ope = 1 - s
            pad += n_ope
        else:  # -にしたい場合
            n_ope = s + 1
            pad -= n_ope
        ans2 += n_ope
    now *= -1

# print(S)
# print(ans1, ans2)
print(min(ans1, ans2))
