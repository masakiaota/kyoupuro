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


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 行が重要, ある行に示す列の内一つ文字を抜き出して回文を作りたい
# i文字目とN-1-i文字目が一致→そのような行の共通要素を抜き出せば良い
# 偶数と奇数で場合わけ

N = read_a_int()
A = read_map(N)

ans = []
for i in range(N // 2):  # 真ん中の手前の行まで
    common = set(list(A[i])) & set(list(A[- 1 - i]))
    if len(common) == 0:
        print(-1)
        exit()
    ans.append(list(common)[0])

if N & 1:  # 奇数の場合
    print(''.join(ans) + A[N // 2][0] + ''.join(reversed(ans)))
else:
    print(''.join(ans) + ''.join(reversed(ans)))
