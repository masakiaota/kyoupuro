import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


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
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


# https://atcoder.jp/contests/diverta2019-2/tasks/diverta2019_2_b
N = a_int()
XY = read_tuple(N)
XY.sort()
if N == 1:
    print(1)
    exit()
# q,pは何通りありえる？N**2//2通りぐらい
# p,qを決定したらあとは直線に乗っているか確かめれば良い
# 点に当たるたびにO(N)かけて調べれば良い
# 以下のO(n**4)のプログラムが可能
# for q,p全通り
#     while ある点を選択
#        その点の+p+qを見てボールを消す
#     コストをmin
# と思ったけど、ただ単純にp,qで拾えるボールの個数を数えればいいかも

PQcandi = []
for po1, po2 in combinations(XY, r=2):
    dx = po1[0] - po2[0]
    dy = po1[1] - po2[1]
    if dx < 0:
        dx *= -1
        dy *= -1
    elif dx == 0:
        dy = abs(dy)

    PQcandi.append((dx, dy))


def del_ball(p, q, XYtmp):
    ret = []
    # x軸にソートしてあるので左から拾っていく
    x, y = XYtmp[0]
    for xx, yy in XYtmp[1:]:
        if xx == x + p and yy == y + q:
            x += p
            y += q
        else:
            ret.append((xx, yy))

    return ret


from copy import deepcopy
ans = INF
for p, q in PQcandi:
    XYtmp = deepcopy(XY)
    anstmp = 0
    while XYtmp:
        anstmp += 1
        XYtmp = del_ball(p, q, XYtmp)
    # print(p, q, anstmp)
    ans = min(ans, anstmp)
print(ans)
