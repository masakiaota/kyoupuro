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

# https://atcoder.jp/contests/jsc2019-qual/tasks/jsc2019_qual_b
# 転倒数はどのように増えるのか？
# もとの配列の転倒数と結合したときに増える転倒数に分解する
# もとの配列の転倒数はただ単純にK回掛ければいい
# 結合したときに増える転倒数はK*(K-1)//2をかける

N, K = read_ints()
A = read_ints()

n_origin = 0  # Aの転倒数
for a, b in combinations(A, r=2):
    if a > b:
        n_origin += 1

n_append = 0  # A+Aを結合したときに転倒数がいくつ増えるか
# これは愚直に各要素にたいして、以下の数がいくつあるか数え上げてもいいが、
# 結合したときの転倒数-2*n_originでも求められそう
for a, b in combinations(A + A, r=2):
    if a > b:
        n_append += 1
n_append -= n_origin * 2
print((n_origin * K + n_append * (K - 1) * K // 2) % MOD)
