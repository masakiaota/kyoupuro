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


def read_map_as(H, replace={'#': 1, '.': 0}, pad=0):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    # TODO paddingの機能入れたいね
    ret = [[pad] * (W + 2)]
    for _ in range(H):
        ret.append([pad] + [replace[s] for s in read()[:-1]] + [pad])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    ret.append([pad] * (W + 2))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc096/tasks/abc096_c
# 普通に上下左右の連結成分が全部2以上か調べれば良い
# というか上下左右の連結成分がない場合に即時終了すれば良い
H, W = read_ints()
S = read_map_as(H)
for i, j in product(ra(1, H + 1), ra(1, W + 1)):
    if S[i][j] == 0:
        continue
    for di, dj in [(0, 1), (1, 0), (-1, 0), (0, -1)]:
        ni, nj = i + di, j + dj
        if S[ni][nj] == 1:
            break
    else:
        print('No')
        exit()

print('Yes')
