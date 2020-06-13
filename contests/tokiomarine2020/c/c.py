import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


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
from operator import itemgetter
from itertools import product, permutations, combinations, accumulate
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N, K = read_ints()
A = read_ints()
# 愚直に実装するとO(N**3)ぐらいO(N√N)以下じゃないと間に合わない
# 一回あたりの操作はO(N)(imos法)
# 操作回数はmin(K,logN)に気づくと通る

for _ in range(min(50, K)):
    imos = [0] * (N + 1)
    for i, d in enu(A):
        imos[max(i - d, 0)] += 1
        imos[min(i + d + 1, N)] -= 1
    A = list(accumulate(imos))[:-1]
print(*A)
