import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map_as(H, replace={'#': 1, '.': 0}, pad=None):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    if pad is None:
        ret = []
        for _ in range(H):
            ret.append([replace[s] for s in read()[:-1]])
            # 内包表記はpypyでは若干遅いことに注意
            # #numpy使うだろうからこれを残しておくけど
    else:  # paddingする
        ret = [[pad] * (W + 2)]  # Wはどっかで定義しておくことに注意
        for _ in range(H):
            ret.append([pad] + [replace[s] for s in read()[:-1]] + [pad])
        ret.append([pad] * (W + 2))

    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


H, W, K = ints()

import numpy as np
C = np.array(read_map_as(H))
ans = 0
for rows in product(range(2), repeat=H):
    C_r = C.copy()
    for i, tf in enu(rows):
        if tf:
            C_r[i, :] = 0
    for cols in product(range(2), repeat=W):
        C_rc = C_r.copy()
        for j, tf in enu(cols):
            if tf:
                C_rc[:, j] = 0
        ans += (C_rc.sum() == K)
print(ans)
