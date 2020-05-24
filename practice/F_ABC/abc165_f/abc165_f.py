# 木によってLISを作り、dfsで抜けるときにLISをその前の状態まで復元する

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_ints(): return list(map(int, read().split()))


def read_a_int(): return int(read())


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


from bisect import bisect_left, bisect_right
from collections import defaultdict
N = read_a_int()
A = read_ints()
U, V = read_col(N - 1)
U = mina(*U)
V = mina(*V)
tree = defaultdict(lambda: [])
for u, v in zip(U, V):
    tree[u].append(v)
    tree[v].append(u)

LIS = []
ans = [0] * N  # 各ノードのlen(LIS)を記録


def dfs(now, p):  # 現在のノード、親
    a = A[now]
    # LISの更新
    idx = bisect_left(LIS, a)
    is_append = False
    if idx == len(LIS):
        LIS.append(a)
        is_append = True
    else:
        old = LIS[idx]   # なんの値だったか持っておく
        LIS[idx] = a  # aに更新

    ans[now] = len(LIS)  # 答えを記録
    # 次のノードを探索
    for to in tree[now]:
        if to == p:
            continue
        dfs(to, now)

    # 抜けるときにLISを復元
    if is_append:
        del LIS[idx]
    else:
        LIS[idx] = old


dfs(0, -1)
print(*ans, sep='\n')
