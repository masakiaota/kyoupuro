# 公式解説で解く
# https://www.youtube.com/watch?v=1V45kF40zHc&list=PLLeJZg4opYKaru-yFYYQmp4GAg4Ewyg8I&index=12&t=0s

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
tree = defaultdict(lambda: [])  # 単方向に直しておく
for i in range(N):
    p = a_int() - 1
    tree[p].append(i)

Q = a_int()
A, B = read_col(Q)
A = mina(*A)
B = mina(*B)

# dfsで初めて訪問した順を記録する
# またそのノード以下にいくつノードがあるのかも記録しておけば、上記で作成した配列のどの範囲に子が格納しているのかわかる
# ハッシュマップをうまく使ってクエリにO(1)で答えられるようにする
nodes = []
widths = {}

cnt = 0  # そのノード下に何個のノードがあるのか算出用


def dfs(u):  # 現在のノード
    nodes.append(u)
    global cnt
    cnt_u = cnt
    for nx in tree[u]:
        cnt += 1
        dfs(nx)
    n_under = cnt - cnt_u
    widths[u] = n_under


dfs(tree[-2][0])

# 位置を管理
to_idx = {}
for i, u in enu(nodes):
    to_idx[u] = i

# クエリに答える
ans = []
for a, b in zip(A, B):
    # aの位置を調べて,bの部下範囲内(bの位置から+widthの範囲)であればYes
    ans.append('Yes' if to_idx[b] < to_idx[a] <=
               to_idx[b] + widths[b] else 'No')
print(*ans, sep='\n')
