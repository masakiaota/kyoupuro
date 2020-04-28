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
# https://atcoder.jp/contests/arc098/tasks/arc098_a

# i番目の人をリーダーにしたときに何人向く方向を変えるかを高速に取得できれば良い
# [0,i)のWの人数+[i+1,)のEの人数がi番目の人をリーダーにしたときに変える個数
# 上記の一行目と二項目はi→i+1のときに逐次計算できる
N = read_a_int()
S = read()[:-1]
n_W = 0
n_E = S.count('E')
ans = 10**6
for s in S:
    if s == 'E':
        n_E -= 1
    ans = min(ans, n_E + n_W)
    if s == 'W':
        n_W += 1
print(ans)
