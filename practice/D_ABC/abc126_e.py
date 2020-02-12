# https://atcoder.jp/contests/abc126/tasks/abc126_e

# 偶奇を当てる問題
# 制約 A_Xi + A_Yi + Zi は偶数

# もしZiが偶数のとき
# A_Xi , A_Yiは偶,偶 or 奇,奇

# もしZiが奇数なら
# A_Xi , A_Yiは偶,奇 or 奇,偶

# A_XiとA_Yiについて符号が違うか同じかをメモしておけば良い？
# 未確定なAiについて魔法を使う必要がある

# Aiのiをノードとして捉えたとき、ノードXj, Yjは'同じまとまり'としてマージし続けられる
# 同じまとまりに対しては一つのAiを確定させれば、そのまとまりすべてのAiが判明する。
# よってdisjointになっている集合の数が答えになる。


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

    def n_disjoint(self):
        # O(N)かかる #いくつのdisjoint setがあるか
        return sum([x < 0 for x in self.parent])


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, M = read_ints()
uf = UnionFind(N)
for m in range(M):
    x, y, z = read_ints()
    uf.unite(x - 1, y - 1)
print(uf.n_disjoint())
