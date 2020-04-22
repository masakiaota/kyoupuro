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

# https://atcoder.jp/contests/https://atcoder.jp/contests/abc060/tasks/abc060_b

A, B, C = read_ints()

# x %B==Cとしたい。そのときxはAの倍数か？
# y*B+CはAの倍数となるときがあるか？と言い換えられる
# もしくは x-C=0(modB)となるようなAの倍数xは存在するか？となる

# わからんとりあえず1000倍ぐらいしておけばどうにかなるっしょ
for i in range(1000):
    if (i * B + C) % A == 0:
        print('YES')
        exit()
print('NO')
