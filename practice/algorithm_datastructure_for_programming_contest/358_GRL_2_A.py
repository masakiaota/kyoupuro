# https://onlinejudge.u-aizu.ac.jp/courses/library/5/GRL/all/GRL_2_A
# 最小全域木、またお前か(プリムのアルゴリズムでできるやつ)
# ここではクラスカルのアルゴリズムを用いて解く。
# アイデアとしては辺の重みの小さい順にgreedyにノード同士をつないでいけば、必ず最小全域木になるでしょうというもの
# ただし木とならないような辺は省かなければ行けない。
# 辺を追加する際に木であることを崩さないように効率よく判定するのにunion-find木を使っている


class UnionFind:
    def __init__(self, N):
        self.N = N  # ノード数
        # 親ノードをしめす。負は自身が親ということ。
        self.parent = [-1] * N  # idxが各ノードに対応。
        # 本で言うrankはこの実装では扱っていない。

    def root(self, A):  # 本で言うfindset
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


Edges = []
# load data
n_V, n_E = list(map(int, input().split()))
for _ in range(n_E):
    s, t, w = list(map(int, input().split()))
    Edges.append((w, s, t))


def kruskal(N, Edges):
    '''
    Nは頂点数、Ndgesは各要素が(w,s,t)を前提としたlist
    '''
    edges = sorted(Edges)
    ret = 0
    union = UnionFind(N)
    n_edges = 0
    for w, s, t in edges:
        if n_edges == N - 1:
            # 全域木になったら早期終了可
            break
        if union.is_in_same(s, t):
            continue
        union.unite(s, t)
        ret += w
        n_edges += 1
    return ret


print(kruskal(n_V, Edges))
