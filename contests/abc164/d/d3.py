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
from math import gcd

S = read()[:-1]

# 2019の倍数である
# 2019 は 10000倍ごとにサイクルが存在する
# →最小の区間はすぐわかる, 連続するときにやっかい
# 連続するときは何回連続したか数えておけば良いのでは？
# 一回最小の区間で置換しよう あと0000とかもやっかいだけどどうしよう→1→9のみなので大丈夫
# 10000のなかで重複するのはない？

x = 2019
multi_x = set()
for i in range(1, 10000):
    tmp = str(x * i)
    if tmp.count('0'):
        continue
    multi_x.add(str(x * i))
print(multi_x)
# print(len(multi_x))

# 2019の倍数になる最小の区間を
# 5桁から8桁見ておけばいいのは重要なヒント
# 8桁切り出しておいてスライド？#中で8桁まで見る

candi = []
for i in range(len(S)):
    for j in range(i + 3, i + 8):
        if j > len(S):
            continue
        if S[i:j] in multi_x:
            candi.append((i, j))
# print(candi)

# 最小の区間は得られた
# 最小の区間同士がくっつくときが厄介しかもオーバーラップしてる
