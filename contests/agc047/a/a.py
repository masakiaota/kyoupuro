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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce


# 画像でおいた自分の解説見て
N = a_int()
B = []
for _ in ra(N):
    A = float(read()[:-1])  # floatなら小数13桁ぐらい入る
    B.append(int(A * (10**9)))

N = []
M = []
for b in B:
    n = 0
    while ~b & 1:  # 偶数である限り
        b >>= 1
        n += 1
    N.append(n)

    m = 0
    while b % 5 == 0:  # 5で割れる限り
        b //= 5
        m += 1
    M.append(m)

cnt = [[0] * 19 for _ in ra(19)]
ans = 0
for n, m in zip(N, M):
    x = 18 - n
    y = 18 - m
    for i, j in product(range(x, 19), range(y, 19)):
        ans += cnt[i][j]

    # このデータをcntに追加
    cnt[min(n, 18)][min(m, 18)] += 1

print(ans)
