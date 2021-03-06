# dfsか？
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
rr = range


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


K = read_a_int()

# bfsで全探索したほうが良くない？
cnt = 0

candi = set()


def dfs(now: int):  # 上の桁からやってく
    if len(str(now)) >= 11:
        return
    candi.add(now)
    right = now % 10  # 下一桁
    # 探索
    if right == 0:
        tmp = [right, right + 1]
    elif right == 9:
        tmp = [right - 1, right]
    else:
        tmp = [right - 1, right, right + 1]
    for ad in tmp:
        dfs(now * 10 + ad)


# print(sorted(candi))
for i in range(1, 10):
    dfs(i)
print(sorted(candi)[K - 1])
