# http://poj.org/problem?id=3723


class UnionFind:
    def __init__(self, N):
        self.N = N  # ノード数
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

        return True

    def is_in_same(self, A, B):
        return self.root(A) == self.root(B)


def kruskal(N, edges):
    '''Nは頂点数,edgesは(長さ,s,t)を要素に持つリスト
    最小全域森のtotal_cost、最小全域森を構成する辺の集合used_edgesを返す'''
    uf = UnionFind(N)
    total_cost = 0
    used_edges = []
    edges = sorted(edges)
    for c, s, t in edges:
        if uf.is_in_same(s, t):
            continue
        uf.unite(s, t)
        total_cost += c
        used_edges.append((c, s, t))
    return total_cost, used_edges


N = 5
M = 5
R = 8

XYD = [(4, 3, 6831),
       (1, 3, 4583),
       (0, 0, 6592),
       (0, 1, 3063),
       (3, 3, 4975),
       (1, 3, 2049),
       (4, 2, 2104),
       (2, 2, 781), ]

# 頭良すぎか？
# 親密度を負のコストだと見立てれば、最小全域木の順番に徴兵するのが一番コストのかからない順番になる。(始めるノードによらない)

edges = []
n_nodes = N + M  # 前N個のノードを男性用にする
for x, y, d in XYD:
    edges.append((-d, x, N + y))

mina_cost, used_relation = kruskal(n_nodes, edges)
print(mina_cost)
print(*used_relation, sep='\n')
print(10000 * n_nodes + mina_cost)
