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

# https://atcoder.jp/contests/abc134/tasks/abc134_d
# 偶奇だけが重要
# 後半は即座に決定できる(倍数が存在しないので)
# 後ろから決定できない？
# nの倍数の個数を更新していく

# 昔はスラスラ解けたのにな... 復習しよう

N = read_a_int()
A = read_ints()

ans = [0] * N
ans[N // 2:] = A[N // 2:]
for i in ra(N // 2 - 1, -1, -1):
    # (i+1)の倍数の個数の集計
    n = i + 1
    s = sum(ans[i::n]) % 2
    ans[i] = s ^ A[i]

print(sum(ans))
ans_pr = []
for i, v in enu(ans, start=1):
    if v:
        ans_pr.append(i)
print(*ans_pr)
