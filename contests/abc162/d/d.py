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


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a * b // g


N = read_a_int()
S = read()[:-1]
# 3文字選んで全部違う文字かつ、同じ分離れてるものじゃない
# 全通り-同じ分離れてる通り

# 全通り→sum_i i文字
# i文字目は何通りできるか？→(i:]文字の中でi文字目と違う文字の数nに対してnC2

# 同じ分離れている通り(削除する分)はどうやって数えるか？
# →O(n^2)かけてスライドさせて見てみるか...
# n_diff = []  # i文字目と違う文字が何文字あるか
n_r = S.count('R')
n_g = S.count('G')
n_b = S.count('B')
tmp = {'R': n_r,
       'G': n_g,
       'B': n_b}
total = 0
for s in S:
    tmp[s] -= 1
    if s == 'R':
        n_diff = tmp['G'] * tmp['B']
    elif s == 'G':
        n_diff = tmp['R'] * tmp['B']
    if s == 'B':
        n_diff = tmp['G'] * tmp['R']
    total += n_diff


# O(nn)かけて条件を満たさない個数を探す
# 条件がないときに違う組み合わせの通りの数はわかっている
# 条件を満たしてしまうときは？組み合わせの中で
rem = 0
for add in ra(1, N + 1):
    if add * 2 >= N:
        break
    for i in ra(N):
        j = i + add
        k = j + add
        if k >= N:
            break
        # if S[i] == S[j] or S[j] == S[k] or S[k] == S[i]:
        if S[i] != S[j] and S[j] != S[k] and S[k] != S[i]:
            rem += 1

print(total - rem)
# print(total, rem)
