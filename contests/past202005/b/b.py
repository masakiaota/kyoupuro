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

N, M, Q = read_ints()
# 結局最後に解かれた人数だけが重要
# 現在の得点だけが重要
problem_scores = [N] * M  # 各問題の現在の得点
solved = [set() for _ in range(N)]  # i番目の人が何番目の問題を解いたか
ans = []
for _ in ra(Q):
    cmd, *tmp = read_ints()
    if cmd == 1:  # スコア出力
        n = tmp[0] - 1
        score = 0
        for m in solved[n]:
            score += problem_scores[m]
        ans.append(score)

    else:  # 問題といた
        n, m = tmp
        m -= 1
        n -= 1
        problem_scores[m] -= 1
        solved[n].add(m)

print(*ans, sep='\n')
