# ダブリングで解く

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


from collections import defaultdict

N = a_int()
P, = read_col(N)
P = mina(*P)
tree = defaultdict(lambda: [])  # 単方向に直しておく
for i, p in enu(P):
    tree[p].append(i)

Q = a_int()
A, B = read_col(Q)
A = mina(*A)
B = mina(*B)


# ダブリングテーブル作成
# parent[k][v]を、vの2**k個上の親ノードと定義する。存在しない時は-1とする
K = 20
parent = [[114514] * N for k in ra(K)]  # 2**19もあれば十分
# 初期値
for u, p in enu(P):
    parent[0][u] = p

for k in ra(K - 1):
    for v in ra(N):
        if parent[k][v] == -2:
            # もう親がない場合
            parent[k + 1][v] = -2
        else:
            parent[k + 1][v] = parent[k][parent[k][v]]


# 各ノードのdepthを取得
D = {}


def dfs(u, d):
    D[u] = d
    for nx in tree[u]:
        dfs(nx, d + 1)


dfs(tree[-2][0], 0)

# クエリに答える
ans = []
for a, b in zip(A, B):
    dd = D[a] - D[b]
    if dd < 0:
        ans.append('No')
        continue
    # aのdd個上を高速取得
    now = a
    for j in ra(dd.bit_length()):  # 2bitのj桁目について
        if (dd >> j) & 1:
            now = parent[j][now]
    ans.append('Yes' if now == b else 'No')
print(*ans, sep='\n')
