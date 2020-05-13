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

# https://atcoder.jp/contests/abc049/tasks/arc065_a
# まえからっ読んでいくと文字列のオーバーラップが多くいし、末尾のオーバーラップも多くてしんどい
# 逆から読んでくと3文字で即時決定できる

S = read()[:-1]
S = deque(reversed('xxxxxxxxxx' + S))  # よしなにpaddingすることで終端処理を楽に

# print(S)
while len(S) > 10:
    end3 = ''.join([S[6], S[5], S[4], S[3], S[2], S[1], S[0]])
    # print(end3)
    if end3[2:] == 'dream':
        num_pop = 5
    elif end3 == 'dreamer':
        num_pop = 7
    elif end3[2:] == 'erase':
        num_pop = 5
    elif end3[1:] == 'eraser':
        num_pop = 6
    else:
        print('NO')
        exit()
    for _ in ra(num_pop):
        S.popleft()

# print(S)
if len(S) == 10:
    print('YES')
else:
    print('NO')
