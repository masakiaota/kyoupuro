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

# https://atcoder.jp/contests/abc111/tasks/arc103_a
# v[::2],v[1::2]に分けて考える
# vの一番多い要素をカウント
# ただし、前者と後者d一番多い要素が等しい場合は、どちらかの二番目を用いる

n = read_a_int()
V = read_ints()

cnt0 = [(0, 0)] + list(Counter(V[::2]).items())
cnt1 = [(0, 0)] + list(Counter(V[1::2]).items())
cnt0.sort(key=itemgetter(1))
cnt1.sort(key=itemgetter(1))

if cnt0[-1][0] == cnt1[-1][0]:
    second = max(cnt0[-2][1], cnt1[-2][1])
    first = max(cnt0[-1][1], cnt1[-1][1])
    print(n - (second + first))
else:
    print(n - (cnt0[-1][1] + cnt1[-1][1]))
