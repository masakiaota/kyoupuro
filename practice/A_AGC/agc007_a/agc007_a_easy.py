# https://atcoder.jp/contests/agc007/tasks/agc007_a
# よく考えてみれば最短距離で進むということは、横にW-1回、縦にH-1回移動するのだから
# #に成るマス目は最初の1マスも含めてH+W-1個であるはずである。
# いま最初と最後は連結であることがわかっているのだから、個数だけ確かめれば良い
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
n_sharp = reduce(add, reduce(add, A))
print('Possible' if n_sharp == H + W - 1 else 'Impossible')
