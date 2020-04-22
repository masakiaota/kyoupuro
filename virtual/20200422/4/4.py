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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc127/tasks/abc127_d
# わからん... データ構造で殴ります...
# いや、書き換えるっていうのは追加するっていうのと同じ意味じゃない？
# カードの枚数が多いから追加するのも大変→カードの枚数で管理しよう
N, M = read_ints()
A = read_ints()
A_cnt = Counter(A)
for _ in ra(M):
    b, c = read_ints()
    A_cnt[c] += b

tmp = list(A_cnt.items())
tmp.sort(key=lambda x: x[0], reverse=True)

ans = 0
cnt = 0
for k, v in tmp:
    while v > 0 and cnt < N:
        ans += k
        v -= 1
        cnt += 1
    if cnt == N:
        break


print(ans)
