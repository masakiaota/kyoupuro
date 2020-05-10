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

N, K = read_ints()

A = read_ints()
B = []
for a in A:
    B.append(a - 1)

# ループするまで回してみる?
# 閉路が知りたい
# 閉路の起点をメモ
# K←（K-起点までの回数）
# K%閉路の長さ
# のこりは愚直に起点から再スタート

nums_to_arrive = [-1] * N
nums_to_arrive[0] = 0
now = 0
while True:
    to = B[now]
    if nums_to_arrive[to] == -1:
        nums_to_arrive[to] = nums_to_arrive[now] + 1
        if nums_to_arrive[to] == K:
            print(to + 1)
            exit()
    else:
        break
    now = to
n1 = nums_to_arrive[to]
n2 = nums_to_arrive[now] + 1 - n1
base = to  # 起点
# print(base)


K = (K - n1) % n2  # 残り移動回数
# print(n1, n2, K)
now = base  # 起点をもとに開始する
while K:
    to = B[now]
    K -= 1
    now = to
print(now + 1)
