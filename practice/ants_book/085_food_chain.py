# http://poj.org/problem?id=1182


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


N = 100
K = 7
TXY = [(1, 101, 1),
       (2, 1, 2),
       (2, 2, 3),
       (2, 3, 3),
       (1, 1, 3),
       (2, 3, 1),
       (1, 5, 5), ]

# 頭良すぎか？
# x-A,x-B,x-Cをx,x+N,x+2Nに対応させる
uf = UnionFind(3 * N)
ans = 0
for t, x, y in TXY:
    x -= 1
    y -= 1
    if t == 1:
        # 矛盾チェック
        if uf.is_in_same(x, y + N) or uf.is_in_same(x, y + 2 * N):
            # もしxとyが別のグループに属しているなら同じ種類ということはできない
            ans += 1
        else:
            uf.unite(x, y)
            uf.unite(x + N, y + N)
            uf.unite(x + 2 * N, y + 2 * N)
    else:
        if uf.is_in_same(x, y) or uf.is_in_same(x, y + 2 * N):
            # もしxとyが同じ種類 か 食べられる関係性が逆の場合は矛盾
            ans += 1
        else:
            uf.unite(x, y + N)
            uf.unite(x + N, y + 2 * N)
            uf.unite(x + 2 * N, y)

print(ans)
