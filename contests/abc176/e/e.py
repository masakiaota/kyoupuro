import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


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


HH, WW, M = ints()
H, W = read_col(M)


def flatten(i, j): return i * WW + j


def reshape(x): return x // WW, x % WW


H = mina(*H)
W = mina(*W)
H_cnt = Counter(H)
W_cnt = Counter(W)
# 爆弾がある座標に関しては-1,それ以外はそのまま
# 爆弾のある座標の具体的な爆破数を持っていれば良い


n_bomb = defaultdict(lambda: 0)
ma = -1
for h, w in zip(H, W):
    n_bomb[flatten(h, w)] = H_cnt[h] + W_cnt[w] - 1
    ma = max(ma, H_cnt[h] + W_cnt[w] - 1)

# 縦横大きい方から見てく
H_rank = H_cnt.most_common()
W_rank = W_cnt.most_common()
# print(H_rank, W_rank)

for i, h_cnt in H_rank:
    for j, w_cnt in W_rank:
        if h_cnt + w_cnt <= ma:
            break  # どうあがいてもこれ以上大きくするのは無理
        n = n_bomb[flatten(i, j)]
        if n == 0:
            n = h_cnt + w_cnt
        ma = max(ma, n)

print(ma)
