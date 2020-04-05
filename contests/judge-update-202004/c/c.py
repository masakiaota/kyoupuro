
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


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


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


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from fractions import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a * b // g


a, b, c = read_ints()
N = a + b + c


# 3**9を全探索すれば良くない？
def fill_masu(ls: list):
    masu = [[10 ** 9] * 4 for _ in range(4)]
    for i in ra(a):
        masu[0][i] = ls.pop()
    for i in ra(b):
        masu[1][i] = ls.pop()
    for i in ra(c):
        masu[2][i] = ls.pop()
    return masu


def check(masu):
    for i in ra(a):
        if masu[0][i] > masu[0][i + 1]:
            return False
        if masu[0][i] > masu[1][i]:
            return False
    for i in ra(b):
        if masu[1][i] > masu[1][i + 1]:
            return False
        if masu[1][i] > masu[2][i]:
            return False
    for i in ra(c):
        if masu[2][i] > masu[2][i + 1]:
            return False
        if masu[2][i] > masu[3][i]:
            return False
    return True


ans = 0
for ls in permutations(range(1, N + 1)):
    masu = fill_masu(list(ls))
    ans += check(masu)
print(ans)
