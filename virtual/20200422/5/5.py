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

# https://atcoder.jp/contests/abc074/tasks/arc083_a
# 要復習
A, B, C, D, E, F = read_ints()


# Aを何回を行うか Bを何回行うかは全探索できる #30通り*30通り(たかだか)
# c,dを何回行うかは事前に作っておけば二分探索でギリギリのを求められる
satou_candi = set()
for c, d in product(range(3000), repeat=2):
    if c * C + d * D > F:
        continue
    satou_candi.add(c * C + d * D)

water_candi = set()
for a, b in product(range(30), repeat=2):
    if 100 * (A * a + B * b) > F:
        continue
    water_candi.add(A * a + B * b)

satou_candi = sorted(satou_candi)
water_candi = sorted(water_candi)

ans = (0, 1)

for water in water_candi:
    max_satou = E * water  # 100lあたり
    idx = bisect_left(satou_candi, max_satou) - 1
    if idx == -1:
        continue
    satou = satou_candi[idx]
    if max_satou + water * 100 > F:
        continue
    if ans[0] / (ans[0] + ans[1] * 100) < satou / (satou + water * 100):
        ans = (satou, water)

print(ans[1] * 100 + ans[0], ans[0])
