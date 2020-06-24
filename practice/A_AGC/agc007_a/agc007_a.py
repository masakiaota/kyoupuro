# https://atcoder.jp/contests/agc007/tasks/agc007_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_ints(): return list(map(int, read().split()))


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

# 常に右、下に動いていたなら、経路は一つに決まる。
# →#をたどってそれ以外に#があるかを判別すれば良い
H, W = read_ints()
A = read_map_as(H)
i, j = 0, 0
A[i][j] = 0
while i != H - 1 or j != W - 1:
    if i + 1 < H and j + 1 < W and A[i + 1][j] and A[i][j + 1]:
        print('Impossible')
        exit()
    ni = i
    nj = j
    if i + 1 < H and A[i + 1][j]:
        ni = i + 1
    if j + 1 < W and A[i][j + 1]:
        nj = j + 1
    if ni == i and nj == j:
        print('Impossible')
        exit()
    A[ni][nj] = 0  # 簡単のために0を埋めておく
    i, j = ni, nj

res = reduce(add, reduce(add, A))
# print(*A, sep='\n')
# print(res)
print('Possible' if res == 0 else 'Impossible')
