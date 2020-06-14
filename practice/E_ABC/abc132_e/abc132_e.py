# https://atcoder.jp/contests/abc132/tasks/abc132_e
# ぱっと思いつくのはダイクストラで3回で到達できる点を探索していくとかかなぁ
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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 距離を3で割ったときに0,1,2余る場合について各ノードの最短距離を求めればよい

N, M = read_ints()
graph = defaultdict(lambda: [])
for _ in range(M):
    u, v = mina(*read_ints())
    graph[u].append(v + N)
    graph[u + N].append(v + 2 * N)
    graph[u + 2 * N].append(v)

S, T = mina(*read_ints())

pq = deque([])
D = [-3] * (N * 3)
D[S] = 0
pq.append((0, S))  # (最短距離, 次のノード)
while pq:
    d, v = pq.popleft()
    for to in graph[v]:
        if D[to] != -3:
            continue
        D[to] = d + 1
        pq.append((D[to], to))

print(D[T] // 3)
