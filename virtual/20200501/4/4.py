
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

# https://atcoder.jp/contests/abc085/tasks/abc085_d
# N本の刀、それぞれ2種の攻撃力
# 最小の攻撃回数は？
# 投げつけるほうが必ず強い
# 一番強い通常攻撃<投げつけは全部使ったほうが得。
# 一番強い通常攻撃より強い投げつけに限定してソート強い方から使っていくのが最適

N, H = read_ints()
A, B = read_col(N)
A_ma = max(A)

B.sort()
B_use = B[bisect_right(B, A_ma):]
B_use = list(reversed(B_use))

ans = 0
dmg = 0
for b in B_use:
    if dmg >= H:
        break
    ans += 1
    dmg += b

H -= dmg
if H > 0:
    ans += (H - 1) // A_ma + 1

print(ans)
