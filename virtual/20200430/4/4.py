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


def read_map_as(H, W, replace={'#': 1, '.': 0}):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    ret = []
    ret.append([1] * (W + 2))
    for _ in range(H):
        ret.append([1] + [replace[s] for s in read()[:-1]] + [1])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    ret.append([1] * (W + 2))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc129/tasks/abc129_d
# すべてのマスにおいてみてって感じかな
# 2000*2000だとマスにおいたときにO(1)で何マス照らせるか取得したい
# 前処理で横/縦に何マス伸びてるか記録しておけばいいのでは？

H, W = read_ints()
S = read_map_as(H, W)

yoko = [[-1] * (W + 2) for _ in ra(H + 2)]
tate = [[-1] * (W + 2) for _ in ra(H + 2)]

# 横の前処理
for i in ra(H + 2):
    fillranges = []
    pre_0 = 0
    for j in ra(1, W + 2):
        if S[i][j] == 1:
            fillranges.append((pre_0, j))
            pre_0 = j
    for l, r in fillranges:
        for j in range(l, r):
            yoko[i][j] = r - l - 1

# 縦の前処理
for j in ra(W + 2):
    fillranges = []
    pre_0 = 0
    for i in ra(1, H + 2):
        if S[i][j] == 1:
            fillranges.append((pre_0, i))
            pre_0 = i
    for l, r in fillranges:
        for i in range(l, r):
            tate[i][j] = r - l - 1

# from pprint import pprint
# pprint(tate)
# pprint(yoko)
ans = 0
for i, j in product(range(1, H + 1), range(1, W + 1)):
    ans = max(yoko[i][j] + tate[i][j] - 1, ans)
print(ans)
