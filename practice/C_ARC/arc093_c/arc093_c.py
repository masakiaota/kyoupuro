# https://atcoder.jp/contests/arc093/tasks/arc093_a
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


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

N = a_int()
A = [0] + ints() + [0]
d_nx = [abs(x - y) for x, y in zip(A, A[1:])]  # 一つとなりへ移動するための距離
# d_nx[i]はA[i]→A[i+1]までの距離
d_nnx = [abs(x - y) for x, y in zip(A, A[2:])]  # 2つ隣に移動するための距離
# d_nnx[i] はA[i]→A[i+2]までの距離
total = sum(d_nx)
ans = []
for i in range(N):
    ans.append(total - (d_nx[i] + d_nx[i + 1]) + d_nnx[i])
print(*ans, sep='\n')
