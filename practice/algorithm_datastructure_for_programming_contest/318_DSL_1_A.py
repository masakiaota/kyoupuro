# https://onlinejudge.u-aizu.ac.jp/courses/library/3/DSL/all/DSL_1_A
# AtCoderの解説がわかりやすい https://youtu.be/zV3Ul2pA2Fw?t=1425
# この解説を見た上で本を読むとより理解が深まる

# 本とはやや違う実装だがこちらのほうがスマート(だとおもってる)
# あとで自分の解説を書き直し


class UnionFind:
    def __init__(self, N):
        self.N = N  # ノード数
        # 親ノードをしめす。負は自身が親ということ。
        self.parent = [-1] * N  # idxが各ノードに対応。
        # rankは親ノードの負の数の大きさに対応している

    def root(self, A):  # 本で言うfindset
        # print(A)
        # ノード番号を受け取って一番上の親ノードの番号を帰す
        if (self.parent[A] < 0):
            return A
        self.parent[A] = self.root(self.parent[A])  # 経由したノードすべての親を上書き
        return self.parent[A]

    def size(self, A):  # 本で言うrank
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


N, Q = list(map(int, input().split()))
ds = UnionFind(N)
for q in range(Q):
    com, x, y = list(map(int, input().split()))
    if com == 0:
        # unite
        ds.unite(x, y)
    elif com == 1:
        # same?
        print(1 if ds.is_in_same(x, y) else 0)
