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

# N,Qはたかだか200→愚直に操作を行えば良さそう
N, M, Q = read_ints()
graph = defaultdict(lambda: [])
for _ in range(M):
    u, v = read_ints()
    u -= 1
    v -= 1
    graph[u].append(v)
    graph[v].append(u)

C = read_ints()  # 色

# print(C)


def update(x):
    # xの隣接をxの色で塗り替える
    c = C[x]
    for to in graph[x]:
        C[to] = c



# queryに愚直に答える
for _ in range(Q):
    cmd, *tmp = read_ints()
    if cmd == 1:  # スプリンクラー
        x = tmp[0] - 1
        print(C[x])
        update(x)
    else:  # 上書き
        x, y = tmp
        x -= 1
        print(C[x])
        C[x] = y
