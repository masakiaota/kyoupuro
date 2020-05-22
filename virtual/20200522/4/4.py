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

# https://atcoder.jp/contests/abc076/tasks/abc076_c
# Tの位置は全探索可能
# その他で?があればすべてaに埋めてしまえばいい

S = input()
T = input()
for i in range(len(S) - len(T), -1, -1):  # iの位置からTの挿入を試みる
    sub = S[i:i + len(T)]
    is_ok = True
    for s, t in zip(sub, T):
        if s == '?' or s == t:
            continue
        else:
            is_ok = False
            break
    if is_ok:
        break
else:
    print('UNRESTORABLE')
    exit()

ans = S[:i] + T + S[i + len(T):]
ans = ans.replace('?', 'a')
print(ans)
