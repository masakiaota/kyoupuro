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

# https://atcoder.jp/contests/abc064/tasks/abc064_c
N = read_a_int()
A = read_ints()

if N == 1:  # コーナーケースかー
    print(1, 1)
    exit()


# 色は400ごとに変化(400で割っておくと処理が楽そう)
import numpy as np
A = np.array(A) // 400

cnt = Counter(A)
# print(cnt)
ans_min = 0
ans_max = 0
for k, v in cnt.items():
    if k >= 8:
        ans_max += v
    else:
        ans_min += 1
        ans_max += 1
if ans_min == 0:
    ans_min = 1  # 全員レッドコーダー
print(ans_min, ans_max)
