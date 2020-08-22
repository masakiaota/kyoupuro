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


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


class UnionFind:
    def __init__(self, N):
        self.N = N  # ノード数
        self.n_groups = N  # グループ数
        # 親ノードをしめす。負は自身が親ということ。
        self.parent = [-1] * N  # idxが各ノードに対応。

    def root(self, A):
        # print(A)
        # ノード番号を受け取って一番上の親ノードの番号を帰す
        if (self.parent[A] < 0):
            return A
        self.parent[A] = self.root(self.parent[A])  # 経由したノードすべての親を上書き
        return self.parent[A]

    def size(self, A):
        # ノード番号を受け取って、そのノードが含まれている集合のサイズを返す。
        return -self.parent[self.root(A)]

    def unite(self, A, B):
        # ノード番号を2つ受け取って、そのノード同士をつなげる処理を行う。
        # 引数のノードを直接つなぐ代わりに、親同士を連結する処理にする。
        A = self.root(A)
        B = self.root(B)

        # すでにくっついている場合
        if (A == B):
            return False

        # 大きい方に小さい方をくっつけたほうが処理が軽いので大小比較
        if (self.size(A) < self.size(B)):
            A, B = B, A

        # くっつける
        self.parent[A] += self.parent[B]  # sizeの更新
        self.parent[B] = A  # self.rootが呼び出されればBにくっついてるノードもすべて親がAだと上書きされる
        self.n_groups -= 1

        return True

    def is_in_same(self, A, B):
        return self.root(A) == self.root(B)


def read_map_as(H, replace={'#': 1, '.': 0}, pad=None):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    if pad is None:
        ret = []
        for _ in range(H):
            ret.append([replace[s] for s in read()[:-1]])
            # 内包表記はpypyでは若干遅いことに注意
            # #numpy使うだろうからこれを残しておくけど
    else:  # paddingする
        ret = [[pad] * (W + 2)]  # Wはどっかで定義しておくことに注意
        for _ in range(H):
            ret.append([pad] + [replace[s] for s in read()[:-1]] + [pad])
        ret.append([pad] * (W + 2))

    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


# ufで連結成分に分ける
# 5*5 でノードをつなげていく
# si,sjの属するノードからti,tjの属するノードまでの最短距離が答え
H, W = ints()
si, sj = mina(*ints())
ti, tj = mina(*ints())
S = read_map_as(H)
uf = UnionFind(H * W)

def to_node(i, j): return i * W + j
def to_idx(node): return node // W, node % W

for i, j in product(range(H), range(W)):
    ij = i * W + j
    if S[i][j] == 1:
        continue
    for di, dj in ((0, 1), (1, 0), (0, -1), (-1, 0)):
        ni, nj = i + di, j + dj
        if not (0 <= ni < H and 0 <= nj < W) or S[ni][nj] == 1:
            continue
        uf.unite(ij, ni * W + nj)

# 5*5でノードをつなげる
graph = defaultdict(lambda: set())
for i, j in product(range(H), range(W)):
    if S[i][j] == 1:
        continue
    thisnode = to_node(i, j)
    for di, dj in product([-2, -1, 0, 1, 2], repeat=2):
        ni = i + di
        nj = j + dj
        if di == dj == 0 or not(0 <= ni < H and 0 <= nj < W) or S[ni][nj] == 1:
            continue
        adj_node = to_node(ni, nj)
        if uf.is_in_same(thisnode, adj_node):
            continue
        a = uf.root(adj_node)
        b = uf.root(thisnode)
        graph[a].add(b)
        graph[b].add(a)

# あとはグラフの問題！
s_node = uf.root(to_node(si, sj))
t_node = uf.root(to_node(ti, tj))

que = deque([(s_node, 0)])
is_visited = [False] * len(uf.parent)

while que:
    u, c = que.popleft()
    if u == t_node:
        exit(c)
    for nx in graph[u]:
        if is_visited[nx]:
            continue
        que.append((nx, c + 1))
        is_visited[nx] = True
print(-1)
