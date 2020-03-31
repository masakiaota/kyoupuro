# https://atcoder.jp/contests/abc040/tasks/abc040_d
# 時系列でソートしてunionfindしておしまいやん
# 意外とめんどくさかった
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


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


# w年以前のものは使わない→w>=yとなる道路は使わない→wが大きいと都市同士はつながってない
# w<yとなる道路は使う

from operator import itemgetter
N, M = read_ints()
ABY = read_tuple(M)
ABY.sort(key=itemgetter(2), reverse=True)

uf = UnionFind(N)

Q = read_a_int()
WVI = []
for i in range(Q):
    v, w = read_ints()
    WVI.append((w, v, i))
WVI.sort(reverse=True, key=itemgetter(0))

ans = {}
j = 0  # ABYのidx
for w, v, i in WVI:
    while j < M and ABY[j][2] > w:
        a, b, y = ABY[j]
        uf.unite(a - 1, b - 1)
        # print(a - 1, b - 1)
        j += 1
    ans[i] = uf.size(v - 1)

for i in range(Q):
    print(ans[i])
