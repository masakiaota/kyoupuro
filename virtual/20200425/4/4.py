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

# https://atcoder.jp/contests/abc064/tasks/abc064_d
# カッコこっかを対応させたい → 前か後ろに追加するだけで良い


def n_fusoku(S, target='('):  # 順方向で書いておく (を先頭に追加すべき個数
    stack = []
    ret = 0
    for s in S:
        if s == target:
            stack.append(s)
        else:
            if stack:
                stack.pop()
            else:
                ret += 1  # 不足分
    return ret


N = read_a_int()
S = input()
n_kakko = n_fusoku(S)
n_kokka = n_fusoku(reversed(S), target=')')
print('(' * n_kakko + S + ')' * n_kokka)
