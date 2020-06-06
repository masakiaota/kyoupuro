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


def minaAB(tmp):
    a, b = tmp
    return a - 1, b - 1


# 素直にNNの行列を作ることは不可能
# 列行がどのように入れ替わったか記録しておけばよいのでは？
N = read_a_int()
rows = list(range(N))
cols = list(range(N))
is_T = 0  # False
Q = read_a_int()
ans = []
for _ in ra(Q):
    cmd, *tmp = read_ints()
    if cmd == 1:  # 行番号swap
        a, b = minaAB(tmp)
        if not is_T:
            rows[a], rows[b] = rows[b], rows[a]
        else:
            cols[a], cols[b] = cols[b], cols[a]
    elif cmd == 2:  # 列番号swap
        a, b = minaAB(tmp)
        if is_T:
            rows[a], rows[b] = rows[b], rows[a]
        else:
            cols[a], cols[b] = cols[b], cols[a]
    elif cmd == 3:  # 転置
        is_T = 1 - is_T
    else:  # 出力
        a, b = minaAB(tmp)
        if not is_T:
            ans.append(N * rows[a] + cols[b])
        else:
            ans.append(N * rows[b] + cols[a])

print(*ans, sep='\n')
