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
satou_candi = set()
for c, d in product(range(3001), repeat=2):
    if c * C + d * D > F:
        continue
    satou_candi.add(c * C + d * D)

water_candi = set()
for a, b in product(range(31), repeat=2):
    if 100 * (A * a + B * b) > F:
        continue
    water_candi.add((A * a + B * b) * 100)

satou_candi = sorted(satou_candi)  # この時点で最大3000通りしか無い
water_candi = sorted(water_candi)
# print(satou_candi)
# print(water_candi)

# 反省、ユニークを取っているときは計算量が落ちて結果的に全探索できる場合がある！
# 全探索できる場合は全探索したほうがよい(実装が簡単なら)

ans_satou = 0
ans_total = A * 100
for water in water_candi[1:]:
    for satou in satou_candi:
        if satou + water > F:
            continue
        max_satou = E * water // 100  # 100lあたり
        if satou > max_satou:
            continue
        if ans_satou / ans_total < satou / (satou + water):
            ans_satou = satou
            ans_total = water + satou

# for water in water_candi[1:]:
#     max_satou = E * water // 100  # 100lあたり
#     idx = bisect_right(satou_candi, max_satou) - 1  # -1にはならないはず [0]→0なので #なんで二分探索だとバグるんだろう...
#     satou = satou_candi[idx]
#     # print(water, satou, max_satou)
#     if satou + water > F:
#         continue
#     if ans_satou / ans_total < satou / (satou + water):
#         ans_satou = satou
#         ans_total = water + satou

print(ans_total, ans_satou)
