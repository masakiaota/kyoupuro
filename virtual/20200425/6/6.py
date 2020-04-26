import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc155/tasks/abc155_e

# dp?
# もしXiが6より大きかったらX(i-1)という上の桁10借りてくればお釣りで枚数が減る
# そうなると上位桁では1枚増えるけどi桁では減る
# 解けなかったね ピッタリ払うときと一枚余分に払うときというアイデアは合ってたけど、一枚余分はdpの数字に反映させるんじゃなくて、前の桁に反映させよう


N = '0' + read()[:-1]
pre_is_kuriagari = False
dp1 = int(N[0])
dp2 = dp1 + 1
for i in ra(1, len(N)):
    n = int(N[i])
    # ピッタリ払うときの最小
    oturi = dp2 + (10 - n)
    pay = dp1 + n
    dp1_new = min(oturi, pay)

    # 1多く払うときの最小
    oturi = dp2 + (10 - (n + 1))
    pay = dp1 + n + 1
    dp2_new = min(oturi, pay)
    dp1, dp2 = dp1_new, dp2_new
print(dp1)
