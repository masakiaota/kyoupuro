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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc087/tasks/arc090_b
# x0を0に固定したときに各点への最短距離が求められて、それが矛盾しなければ良さそう
# 任意の点から初めて距離を埋めていく

N, M = read_ints()
LRD = []
graph = defaultdict(lambda: [])
for _ in range(M):
    l, r, d = read_ints()
    l -= 1
    r -= 1
    LRD.append((l, r, d))
    graph[l].append((r, d))
    graph[r].append((l, -d))


D = [INF] * N  # 各点の位置


def bfs(start):
    now, dist = start, 0
    que = deque([(now, dist)])
    D[now] = dist
    while que:
        now, dist = que.popleft()
        for to, co in graph[now]:
            to_dist = dist + co
            if D[to] != INF:
                if D[to] != to_dist:
                    print("No")
                    exit()
                continue
            D[to] = to_dist
            que.append((to, to_dist))


for i in range(N):
    if D[i] != INF:
        continue
    bfs(i)

print('Yes')
