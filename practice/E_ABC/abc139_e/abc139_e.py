# https://atcoder.jp/contests/abc139/tasks/abc139_e
# 詳しくは図の解説を。シミュレーションを高速化するか、トポロジカルソートをうまく使うか
# ここでは練習のためトポロジカルソートをうまく使うことにする

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
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

N = read_a_int()
A = read_matrix(N)
A = [mina(*a) for a in A]  # 0based indexに

# 試合idを決定する
matchID = {}
cnt = 0
for i, j in combinations(range(N), r=2):
    matchID[i, j] = cnt
    matchID[j, i] = cnt  # 一応逆に入力されても大丈夫なように
    cnt += 1

# 入力をmatchID形式に変換
for i, j in product(range(N), range(N - 1)):
    A[i][j] = matchID[i, A[i][j]]

# DAGを作る
graph = defaultdict(lambda: [])
indeg = [0] * cnt
for a in A:
    for i in range(N - 2):
        graph[a[i]].append(a[i + 1])
        indeg[a[i + 1]] += 1

# トポロジカルソートの容量ですべての試合の日数を割り振る
day = [-1] * cnt  # 何日目にその試合が行われたか
is_visited = [False] * cnt
que = deque()  # 各要素は(ノード, 到達可能な最短日数) #流入量0になったノードから加えていく
for i in range(cnt):  # 初期化
    if indeg[i] == 0:
        is_visited[i] = True
        que.append((i, 1))

while que:
    u, d = que.popleft()
    day[u] = d  # 流入量0になったときのdなので
    for to in graph[u]:
        indeg[to] -= 1
        if indeg[to] or is_visited[to]:  # 流入待ちがまだあるor訪問済みならやる必要はない
            continue
        que.append((to, d + 1))
        is_visited[to] = True

print(-1 if -1 in day else max(day))
