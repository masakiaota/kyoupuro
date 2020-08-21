# DAGの最長経路...トポロジカルソートもどきか？
# 普通にDAGに沿ったdpをすればいいのでは?
# 再帰だと実装楽そうだけどどうしよう
# 問題は終了と最初がわからんこと
# いや最初は全部長さ0をセットすればいいだろ
import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def ints(): return list(map(int, readline().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce

N, M = ints()
dag = defaultdict(lambda: [])
indeg = [0] * N
for _ in ra(M):
    x, y = mina(*ints())
    indeg[y] += 1
    dag[x].append(y)

q = deque()
for u, inn in enu(indeg):  # 多点DFS初期化
    if inn == 0:
        q.append(u)  # 別にlengthの情報をもたせちゃってもいいんだけどね

dp = [0] * N
while q:  # トポロジカル順序でdpテーブルを埋めていく
    u = q.popleft()
    for nx in dag[u]:
        dp[nx] = max(dp[nx], dp[u] + 1)
        indeg[nx] -= 1
        if indeg[nx] == 0:
            q.append(nx)
print(max(dp))
